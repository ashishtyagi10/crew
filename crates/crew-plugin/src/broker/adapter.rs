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

/// A registered agent the broker can address by name.
pub trait Adapter: Send + Sync {
    /// The name messages are addressed to (lowercase, e.g. `"claude"`).
    fn name(&self) -> &str;
    /// Whether this agent's CLI is installed and usable on this machine.
    fn probe(&self) -> bool;
    /// Send `body` to the agent and return its normalized reply, or an error
    /// string (launch failure / timeout) the broker can log.
    fn call(&self, body: &str, timeout: Duration) -> Result<String, String>;
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
mod tests {
    use super::*;

    #[test]
    fn raw_normalize_trims() {
        assert_eq!(Normalize::Raw.apply("  PONG\n"), "PONG");
    }

    #[test]
    fn build_args_substitutes_body() {
        let a = CliAdapter {
            name: "x".into(),
            program: "echo".into(),
            args: vec!["-p".into(), "{}".into()],
            normalize: Normalize::Raw,
        };
        assert_eq!(a.build_args("hi there"), vec!["-p", "hi there"]);
    }

    #[test]
    fn cli_adapter_calls_real_process() {
        // `cat` echoes its arg back via a shell so we exercise call()+normalize.
        let a = CliAdapter {
            name: "echoer".into(),
            program: "sh".into(),
            args: vec!["-c".into(), "printf %s \"$0\"".into(), "{}".into()],
            normalize: Normalize::Raw,
        };
        assert_eq!(a.call("hello", Duration::from_secs(5)).unwrap(), "hello");
    }
}
