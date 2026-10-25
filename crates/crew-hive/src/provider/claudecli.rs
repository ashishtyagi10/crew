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
//! (`--tools ""`), and so are the person's MCP servers (`--strict-mcp-config`
//! with no `--mcp-config`): this is a model, not an agent. `--tools ""` alone
//! left every MCP server on, and a specialist asked for a kernel version ran
//! its command through the person's Node REPL server, past crew's approval
//! gate (live, 2026-10-24). Crew's own tools reach it through the swarm's text
//! convention, since `supports_tools` stays false.
//! Sessions are not persisted; a multi-turn request is flattened into one
//! prompt.
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use super::claudestream::{parse_line, Fold};
use super::{
    request_timeout, ChunkFn, Completion, CompletionRequest, Provider, ProviderError, Turn,
};

#[path = "claudecliexec.rs"]
mod exec;
use exec::{finish, run};

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
    /// partial messages, no built-in tools and no MCP servers, no session
    /// left behind, the request's model, and its system prompt when it has
    /// one.
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
            "--strict-mcp-config".into(),
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
            Turn::User(text) => {
                out.push_str("\n\n[user]\n");
                out.push_str(text);
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
