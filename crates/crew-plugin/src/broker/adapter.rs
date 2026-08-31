//! The agent abstraction. An [`Adapter`] knows one agent's headless command and
//! how to turn its raw stdout into a clean reply; the broker only ever sees the
//! normalized string. [`CliAdapter`] covers any agent driven by a single CLI
//! invocation, which is all three of claude/codex/opencode.
use std::time::Duration;

use super::normalize::opencode_json;
use super::run::{on_path, run_cli};

/// How an agent CLI's stdout becomes a reply string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Normalize {
    /// stdout already is the reply (claude `-p`, codex `exec`); just trim it.
    Raw,
    /// opencode `--format json`: parse the event stream for assistant text.
    OpencodeJson,
}

impl Normalize {
    pub fn apply(self, raw: &str) -> String {
        match self {
            Normalize::Raw => raw.trim().to_string(),
            Normalize::OpencodeJson => opencode_json(raw),
        }
    }
}

/// Real token usage from one API reply, as the provider reported it.
/// `input_tokens` is the call's full prompt size — the agent's live context
/// fill. All zeros when the backend can't report usage (external CLIs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Micro-USD for this reply: the provider's exact figure when reported,
    /// else the pricing-table estimate for the adapter's model; 0 = unknown.
    pub cost_microusd: u64,
}

/// The two live signals of one in-flight hop, passed together so each call
/// site threads one value instead of two. Both are ADVISORY — the end-of-hop
/// reply and its `Stats` stay authoritative — so a backend that cannot stream
/// simply never calls them.
#[derive(Clone)]
pub struct HopStream {
    /// Running chars/4 OUTPUT-token estimate for this hop.
    pub on_tokens: std::sync::Arc<dyn Fn(u64) + Send + Sync>,
    /// Each raw text fragment as it arrives, already agent-scoped.
    pub on_text: std::sync::Arc<dyn Fn(&str) + Send + Sync>,
}

impl HopStream {
    /// Discards both signals — for call paths that never dial an agent and
    /// for tests that don't care about liveness.
    pub fn noop() -> Self {
        Self {
            on_tokens: std::sync::Arc::new(|_| {}),
            on_text: std::sync::Arc::new(|_| {}),
        }
    }
}

/// A registered agent the broker can address by name.
pub trait Adapter: Send + Sync {
    /// The name messages are addressed to (lowercase, e.g. `"claude"`).
    fn name(&self) -> &str;
    /// The model this agent runs on, for roster badges. Empty when the agent
    /// picks its own model (external CLIs).
    fn model(&self) -> &str {
        ""
    }
    /// A short capability hint for the roster and peer lists. Defaults to the
    /// static mapping for the known agent names; manifest plugin agents carry
    /// their own.
    fn role(&self) -> &str {
        super::agents::role_for(self.name())
    }
    /// Whether this agent's CLI is installed and usable on this machine.
    fn probe(&self) -> bool;
    /// Send `body` to the agent and return its normalized reply, or an error
    /// string (launch failure / timeout) the broker can log.
    fn call(&self, body: &str, timeout: Duration) -> Result<String, String>;
    /// Like [`Adapter::call`], also reporting the reply's real token usage.
    /// Defaults to zero usage for backends that can't report it.
    fn call_with_usage(&self, body: &str, timeout: Duration) -> Result<(String, Usage), String> {
        self.call(body, timeout).map(|t| (t, Usage::default()))
    }
    /// Like `call_with_usage`, also reporting this hop's live signals — a
    /// running OUTPUT-token estimate and each streamed text fragment — while
    /// the reply arrives. Default: no live signals (external CLIs return one
    /// blob and have nothing incremental to forward).
    fn call_with_usage_ticked(
        &self,
        body: &str,
        timeout: Duration,
        stream: &HopStream,
    ) -> Result<(String, Usage), String> {
        let _ = stream;
        self.call_with_usage(body, timeout)
    }
}

/// An agent driven by one CLI command. `args` may contain the placeholder
/// `"{}"`, replaced by the message body at call time (so the body is passed as
/// an argument, never piped as raw chatter into another invocation).
pub struct CliAdapter {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    pub normalize: Normalize,
}

impl CliAdapter {
    fn build_args(&self, body: &str) -> Vec<String> {
        self.args.iter().map(|a| a.replace("{}", body)).collect()
    }
}

impl Adapter for CliAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn probe(&self) -> bool {
        on_path(&self.program)
    }

    fn call(&self, body: &str, timeout: Duration) -> Result<String, String> {
        let args = self.build_args(body);
        let raw = run_cli(&self.program, &args, timeout)?;
        Ok(self.normalize.apply(&raw))
    }
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
