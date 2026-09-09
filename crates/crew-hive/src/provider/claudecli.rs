//! A `Provider` over Claude Code's own headless mode: every completion is one
//! `claude -p <prompt> --output-format json` run under the user's Claude
//! Code login. This is how a Claude Pro/Max plan serves the API-shaped seam
//! — the planner, judges and fan-out specialists — the way the CLI relay
//! already serves plain replies: the subscription is used INSIDE the client
//! it is licensed to, and crew drives that client's documented flags. No
//! token is ever read or forwarded.
//!
//! Claude Code's built-in tools are switched off (`--tools ""`): this is a
//! model, not an agent. Crew's own tools reach it through the swarm's text
//! convention, since `supports_tools` stays false. Sessions are not
//! persisted; a multi-turn request is flattened into one prompt.
use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;

use super::{request_timeout, Completion, CompletionRequest, Provider, ProviderError, Turn};

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

/// The subset of the JSON envelope crew reads (`claude` 2.x, 2026-09).
#[derive(Deserialize)]
struct Envelope {
    #[serde(default)]
    is_error: bool,
    #[serde(default)]
    subtype: String,
    #[serde(default)]
    result: Option<String>,
    usage: Option<Usage>,
}

#[derive(Deserialize, Default)]
struct Usage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
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
    /// print mode, JSON out, no built-in tools, no session left behind, the
    /// request's model, and its system prompt when it has one.
    pub fn args(req: &CompletionRequest) -> Vec<String> {
        let mut v = vec![
            "-p".to_string(),
            prompt_text(req),
            "--output-format".into(),
            "json".into(),
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

    /// Read one run: the JSON envelope's `result` is the reply; `is_error`
    /// (or a non-success subtype) is the CLI's own refusal — surfaced as an
    /// API error so the user reads the sentence, not the envelope.
    pub fn parse_result(stdout: &str) -> Result<Completion, ProviderError> {
        let e: Envelope = serde_json::from_str(stdout.trim())
            .map_err(|err| ProviderError::Decode(format!("claude -p output: {err}")))?;
        let text = e.result.unwrap_or_default();
        if e.is_error || (!e.subtype.is_empty() && e.subtype != "success") {
            let why = if text.trim().is_empty() {
                format!("claude exited with {}", e.subtype)
            } else {
                text
            };
            return Err(ProviderError::Api(
                serde_json::json!({"error": {"message": why}}).to_string(),
            ));
        }
        let usage = e.usage.unwrap_or_default();
        Ok(Completion {
            text,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            // The plan covers it; a list-price figure here would show a
            // bill the user never gets.
            cost_microusd: 0,
            calls: Vec::new(),
            thought: String::new(),
        })
    }
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

impl Provider for ClaudeCliProvider {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let program = self.program.clone();
        let timeout = self.timeout;
        let args = Self::args(&req);
        Box::pin(async move {
            let mut std_cmd = std::process::Command::new(&program);
            crate::childproc::no_console_window(&mut std_cmd);
            std_cmd
                .args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut cmd = tokio::process::Command::from(std_cmd);
            cmd.kill_on_drop(true);
            let child = cmd
                .spawn()
                .map_err(|e| ProviderError::Http(format!("failed to launch {program}: {e}")))?;
            let out = tokio::time::timeout(timeout, child.wait_with_output())
                .await
                .map_err(|_| ProviderError::Http(format!("{program} timed out")))?
                .map_err(|e| ProviderError::Http(format!("{program}: {e}")))?;
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let why = stderr
                    .trim()
                    .lines()
                    .last()
                    .unwrap_or("no output")
                    .to_string();
                return Err(ProviderError::Api(
                    serde_json::json!({"error": {"message": format!("{program}: {why}")}})
                        .to_string(),
                ));
            }
            Self::parse_result(&stdout)
        })
    }
}
