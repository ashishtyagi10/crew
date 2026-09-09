//! The `claude` relay agent, LIVE: one `claude -p <body> --output-format
//! stream-json` run per call, read line by line (`crew_hive::claudestream`)
//! so the pane sees the model think, write and call its tools while the
//! task runs, instead of a blank 180 s and a timeout. Claude Code's own
//! tools stay ON — this is an agent working in the pane's directory, and
//! the tool lines are the feedback the user asked for.
//!
//! The deadline is an IDLE one: it restarts on every line the CLI prints.
//! A task that reads a repo for ten minutes is alive the whole time; a CLI
//! that goes silent for the whole window is hung and is killed. The window
//! is the broker's call timeout with a floor (`IDLE_FLOOR`), because one
//! long-running tool call (a build, a test run) prints nothing until it
//! returns.
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crew_hive::childproc::no_console_window;
use crew_hive::claudestream::{parse_line, Fold, StreamEvent};
use crew_hive::{AgentId, HiveEvent};

use super::adapter::{Adapter, HopStream, Usage};
use super::run::{explain_failure, on_path};

/// The least idle time a streaming agent is granted, whatever the call
/// timeout says: a tool call prints nothing while it runs.
const IDLE_FLOOR: Duration = Duration::from_secs(600);

pub struct ClaudeAgent {
    pub program: String,
    pub model: Option<String>,
    /// Test seam: the idle floor (`IDLE_FLOOR` in the real adapter).
    pub idle_floor: Duration,
}

impl ClaudeAgent {
    pub fn new(model: Option<String>) -> Self {
        Self {
            program: "claude".into(),
            model: model.filter(|m| !m.is_empty()),
            idle_floor: IDLE_FLOOR,
        }
    }

    /// The argv one call becomes: print mode with the body as an ARGUMENT
    /// (never piped as chatter), the streamed envelope (`--verbose` is what
    /// lets `-p` stream; partial messages carry the deltas), the model when
    /// one is pinned. No `--tools ""`: the agent keeps its hands.
    pub fn args(&self, body: &str) -> Vec<String> {
        let mut v = vec![
            "-p".to_string(),
            body.to_string(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--include-partial-messages".into(),
        ];
        if let Some(m) = &self.model {
            v.push("--model".into());
            v.push(m.clone());
        }
        v
    }

    fn run(
        &self,
        body: &str,
        timeout: Duration,
        stream: &HopStream,
    ) -> Result<(String, Usage), String> {
        let program = &self.program;
        let mut child = no_console_window(&mut Command::new(program))
            .args(self.args(body))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to launch {program}: {e}"))?;
        let stdout = child.stdout.take().expect("stdout was piped");
        let (tx, rx) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        // Drained on its own thread so a chatty CLI can never fill the pipe.
        let stderr = child.stderr.take().expect("stderr was piped");
        let (etx, erx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut s = String::new();
            let mut r = stderr;
            let _ = r.read_to_string(&mut s);
            let _ = etx.send(s);
        });

        let idle = timeout.max(self.idle_floor);
        let agent = AgentId::minted("claude");
        let mut fold = Fold::default();
        let mut open: HashMap<String, (String, Instant)> = HashMap::new();
        let mut chars = 0u64;
        let mut last = Instant::now();
        loop {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(line) => {
                    last = Instant::now();
                    let ev = parse_line(&line);
                    match ev.as_ref() {
                        Some(StreamEvent::Text(t)) => {
                            (stream.on_text)(t);
                            chars += t.chars().count() as u64;
                            (stream.on_tokens)(chars / 4);
                        }
                        Some(StreamEvent::Thought(t)) => (stream.on_thought)(t),
                        Some(StreamEvent::ToolUse { id, name, input }) => {
                            open.insert(id.clone(), (name.clone(), Instant::now()));
                            (stream.on_tool)(HiveEvent::ToolCall {
                                agent: agent.clone(),
                                label: name.clone(),
                                args: input.clone(),
                            });
                        }
                        Some(StreamEvent::ToolResult { id, ok, text }) => {
                            let (label, t0) = open
                                .remove(id)
                                .unwrap_or_else(|| ("tool".into(), Instant::now()));
                            (stream.on_tool)(HiveEvent::ToolResult {
                                agent: agent.clone(),
                                label,
                                ok: *ok,
                                text: text.clone(),
                                ms: t0.elapsed().as_millis() as u64,
                            });
                        }
                        _ => {}
                    }
                    fold.push(ev.as_ref(), &line);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if last.elapsed() >= idle {
                        let _ = child.kill();
                        let _ = child.wait();
                        (stream.on_thought)("");
                        return Err(format!("{program}: no output for {idle:?} \u{2014} killed"));
                    }
                }
            }
        }
        let _ = child.wait();
        (stream.on_thought)(""); // the hop is over: flush the gate's tail
        let stderr = erx
            .recv_timeout(Duration::from_millis(250))
            .unwrap_or_default();
        finish(fold, program, &stderr)
    }
}

/// The reply a folded run is: the CLI's own refusal reads as the error it
/// printed; an empty run is explained from stderr (`run::explain_failure`).
fn finish(fold: Fold, program: &str, stderr: &str) -> Result<(String, Usage), String> {
    if let Some(r) = fold.result.as_ref() {
        if r.is_error || (!r.subtype.is_empty() && r.subtype != "success") {
            let text = r.text.trim();
            return Err(if text.is_empty() {
                format!("{program} exited with {}", r.subtype)
            } else {
                format!("{program}: {text}")
            });
        }
    }
    let text = fold.reply();
    if text.is_empty() {
        return Err(explain_failure(program, stderr));
    }
    let r = fold.result.unwrap_or_default();
    Ok((
        text,
        Usage {
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            // The plan covers it; a list-price figure would show a bill
            // the user never gets.
            cost_microusd: 0,
        },
    ))
}

impl Adapter for ClaudeAgent {
    fn name(&self) -> &str {
        "claude"
    }

    fn model(&self) -> &str {
        self.model.as_deref().unwrap_or("")
    }

    fn probe(&self) -> bool {
        on_path(&self.program)
    }

    fn call(&self, body: &str, timeout: Duration) -> Result<String, String> {
        self.run(body, timeout, &HopStream::noop()).map(|(t, _)| t)
    }

    fn call_with_usage(&self, body: &str, timeout: Duration) -> Result<(String, Usage), String> {
        self.run(body, timeout, &HopStream::noop())
    }

    fn call_with_usage_ticked(
        &self,
        body: &str,
        timeout: Duration,
        stream: &HopStream,
    ) -> Result<(String, Usage), String> {
        self.run(body, timeout, stream)
    }
}

#[cfg(test)]
#[path = "claudeagent_tests.rs"]
mod tests;
