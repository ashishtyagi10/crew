//! Running the CLI, and reading what it printed back as a completion.
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

use crate::provider::claudestream::{parse_line, Fold, StreamEvent};
use crate::provider::{Chunk, ChunkFn, Completion, ProviderError};

/// The completion a folded run is — or the refusal it was.
pub(super) fn finish(fold: Fold, stderr: &str) -> Result<Completion, ProviderError> {
    let refusal = |why: String| {
        ProviderError::Api(serde_json::json!({"error": {"message": why}}).to_string())
    };
    if let Some(r) = fold.result.as_ref() {
        if r.is_error || (!r.subtype.is_empty() && r.subtype != "success") {
            let text = r.text.trim();
            return Err(refusal(if text.is_empty() {
                format!("claude exited with {}", r.subtype)
            } else {
                text.to_string()
            }));
        }
    }
    let text = fold.reply();
    if text.is_empty() {
        let why = stderr.trim().lines().last().unwrap_or("no output");
        return Err(refusal(format!("claude: {why}")));
    }
    let usage = fold.result.as_ref().cloned().unwrap_or_default();
    Ok(Completion {
        text,
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        // The plan covers it; a list-price figure here would show a bill
        // the user never gets.
        cost_microusd: 0,
        calls: Vec::new(),
        thought: fold.thought,
        truncated: usage.truncated,
        ..Default::default()
    })
}

/// Spawn the CLI and fold its stream line by line, handing each text and
/// thinking delta to `on_chunk` as it lands. One deadline for the whole run
/// (a model call, not an agent's task).
pub(super) async fn run(
    program: String,
    args: Vec<String>,
    timeout: Duration,
    on_chunk: Option<ChunkFn>,
) -> Result<Completion, ProviderError> {
    let mut std_cmd = std::process::Command::new(&program);
    crate::childproc::no_console_window(&mut std_cmd);
    std_cmd
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut cmd = tokio::process::Command::from(std_cmd);
    cmd.kill_on_drop(true);
    let mut child = cmd
        .spawn()
        .map_err(|e| ProviderError::Http(format!("failed to launch {program}: {e}")))?;
    let stdout = child.stdout.take().expect("stdout was piped");
    let mut stderr = child.stderr.take().expect("stderr was piped");
    // Drained alongside stdout so a chatty CLI can never fill the pipe.
    let err_task = tokio::spawn(async move {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s).await;
        s
    });
    let fold_all = async {
        let mut fold = Fold::default();
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let ev = parse_line(&line);
            if let Some(on) = on_chunk.as_ref() {
                match ev.as_ref() {
                    Some(StreamEvent::Text(t)) => on(Chunk::Text(t)),
                    Some(StreamEvent::Thought(t)) => on(Chunk::Thought(t)),
                    _ => {}
                }
            }
            fold.push(ev.as_ref(), &line);
        }
        let _ = child.wait().await;
        fold
    };
    let fold = tokio::time::timeout(timeout, fold_all)
        .await
        .map_err(|_| ProviderError::Http(format!("{program} timed out")))?;
    let stderr = err_task.await.unwrap_or_default();
    finish(fold, &stderr)
}
