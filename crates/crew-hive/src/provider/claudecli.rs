//! A `Provider` over Claude Code's own headless mode: every completion is one
//! `claude -p <prompt> --output-format stream-json` run under the user's
//! Claude Code login. This is how a Claude Pro/Max plan serves the API-shaped
//! seam — the planner, judges and fan-out specialists — the way the CLI relay
//! already serves plain replies: the subscription is used INSIDE the client
//! it is licensed to, and crew drives that client's documented flags. No
//! token is ever read or forwarded.
//!
//! The run STREAMS (`claudestream`): each text and thinking delta reaches
//! `on_chunk` as it arrives, so the pane shows the model working instead of
//! a silent minute. Claude Code's built-in tools are switched off
//! (`--tools ""`): this is a model, not an agent. Crew's own tools reach it
//! through the swarm's text convention, since `supports_tools` stays false.
//! Sessions are not persisted; a multi-turn request is flattened into one
//! prompt.
use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

use super::claudestream::{parse_line, Fold, StreamEvent};
use super::{
    request_timeout, Chunk, ChunkFn, Completion, CompletionRequest, Provider, ProviderError, Turn,
};

#[derive(Clone)]
pub struct ClaudeCliProvider {
    program: String,
    timeout: Duration,
}

impl Default for ClaudeCliProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeCliProvider {
    /// The `claude` on `$PATH`, with the provider-wide request timeout.
    pub fn new() -> Self {
        Self {
            program: "claude".into(),
            timeout: request_timeout(),
        }
    }

    /// Another binary (a test's fake) in place of `claude`.
    pub fn with_program(mut self, program: &str) -> Self {
        self.program = program.into();
        self
    }

    /// The argv one request becomes — pure, so the contract is testable:
    /// print mode, the streamed JSON envelope (which needs `--verbose`) with
    /// partial messages, no built-in tools, no session left behind, the
    /// request's model, and its system prompt when it has one.
    pub fn args(req: &CompletionRequest) -> Vec<String> {
        let mut v = vec![
            "-p".to_string(),
            prompt_text(req),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--include-partial-messages".into(),
            "--tools".into(),
            String::new(),
            "--no-session-persistence".into(),
        ];
        if !req.model.is_empty() {
            v.push("--model".into());
            v.push(req.model.clone());
        }
        if let Some(sys) = req.system.as_deref().filter(|s| !s.trim().is_empty()) {
            v.push("--system-prompt".into());
            v.push(sys.to_string());
        }
        v
    }

    /// Read one whole run's stdout (every line at once — the non-streamed
    /// twin of the live loop, and what the tests feed): the `result` line's
    /// reply and usage; `is_error` (or a non-success subtype) is the CLI's
    /// own refusal — surfaced as an API error so the user reads the
    /// sentence, not the envelope.
    pub fn parse_result(stdout: &str) -> Result<Completion, ProviderError> {
        let mut fold = Fold::default();
        // One object over several lines (a pretty-printed envelope) is
        // one event; otherwise the stream is one event per line.
        if let Some(ev) = parse_line(stdout) {
            fold.push(Some(&ev), stdout);
        } else {
            for line in stdout.lines() {
                fold.push(parse_line(line).as_ref(), line);
            }
        }
        finish(fold, "")
    }
}

/// The completion a folded run is — or the refusal it was.
fn finish(fold: Fold, stderr: &str) -> Result<Completion, ProviderError> {
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
    })
}

/// The one prompt a run gets: the opening message, then any later turns as
/// a transcript — the tool loop only ever reaches a provider that speaks
/// native tools, so this is a safety net, not the common path.
pub fn prompt_text(req: &CompletionRequest) -> String {
    if req.turns.is_empty() {
        return req.prompt.clone();
    }
    let mut out = req.prompt.clone();
    for t in &req.turns {
        match t {
            Turn::Assistant { text, calls } => {
                out.push_str("\n\n[assistant]\n");
                out.push_str(text);
                for c in calls {
                    out.push_str(&format!("\n@{} {}", c.name, c.input));
                }
            }
            Turn::ToolResults(results) => {
                out.push_str("\n\n[tool results]");
                for r in results {
                    out.push_str(&format!("\n{}: {}", r.id, r.content));
                }
            }
        }
    }
    out.push_str("\n\n[assistant]\n");
    out
}

/// Spawn the CLI and fold its stream line by line, handing each text and
/// thinking delta to `on_chunk` as it lands. One deadline for the whole run
/// (a model call, not an agent's task).
async fn run(
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

impl Provider for ClaudeCliProvider {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        Box::pin(run(
            self.program.clone(),
            Self::args(&req),
            self.timeout,
            None,
        ))
    }

    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: ChunkFn,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        Box::pin(run(
            self.program.clone(),
            Self::args(&req),
            self.timeout,
            Some(on_chunk),
        ))
    }
}
