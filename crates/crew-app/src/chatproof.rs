//! Whether a turn that changed files was checked: the evidence, not the claim.
//!
//! An answer can say "the tests pass" whether or not anything ran. A senior
//! engineer reading it looks for the run. So does the pane now: it hears
//! every edit and every command the turn makes, Claude Code's (hive events)
//! and crew's own agents' (`[tool]` cards) alike, and when a turn that edited
//! files ends it says what the last build or test after the last edit did:
//! passed, failed, or never happened. An edit after a passing run makes it
//! not checked again: what was proven is no longer what is on disk.
//!
//! The project's own `.crew/check` outranks this: when it reports in the
//! same turn, its verdict is the one, and this stays quiet.
use crew_hive::HiveEvent;
use crew_plugin::PluginEvent;

use crate::chatphase::{of_tool, Phase};

/// Words that make a command a build, a test or a check.
const VERIFIES: &[&str] = &[
    "test",
    "tests",
    "check",
    "build",
    "clippy",
    "lint",
    "pytest",
    "jest",
    "vitest",
    "tsc",
    "mypy",
    "ruff",
    "eslint",
    "make",
    "ctest",
    "rspec",
    "gradle",
    "mvn",
    "typecheck",
];

/// Chars of a command the verdict quotes.
const CMD_MAX: usize = 60;

/// One turn's evidence.
#[derive(Clone, Debug, Default)]
pub(crate) struct Proof {
    edited: bool,
    /// The last build/test after the last edit, and how it ended (`None`
    /// while it runs).
    run: Option<(String, Option<bool>)>,
    /// The project's own check reported this turn.
    checked: bool,
}

impl Proof {
    /// Fold in one event of the turn.
    pub(crate) fn hear(&mut self, ev: &PluginEvent) {
        match ev {
            PluginEvent::Hive {
                event: HiveEvent::ToolCall { label, args, .. },
            } => self.call(label, &command_of(args)),
            PluginEvent::Hive {
                event: HiveEvent::ToolResult { label, ok, .. },
            } => self.result(label, *ok),
            PluginEvent::Message { text, .. } if text.starts_with("check: ") => self.checked = true,
            PluginEvent::Message { text, .. } => {
                let Some(card) = text.strip_prefix(crate::chatcard::TOOL_PREFIX) else {
                    return;
                };
                let first = card.lines().next().unwrap_or_default();
                let (label, rest) = first.split_once(' ').unwrap_or((first, ""));
                match (first.contains('\u{2713}'), first.contains('\u{2717}')) {
                    (false, false) => self.call(label, rest.trim()),
                    (ok, _) => self.result(label, ok),
                }
            }
            _ => {}
        }
    }

    fn call(&mut self, label: &str, command: &str) {
        match of_tool(label) {
            Phase::Rewriting => (self.edited, self.run) = (true, None),
            Phase::Compiling if verifies(command) => {
                self.run = Some((clip(command), None));
            }
            _ => {}
        }
    }

    fn result(&mut self, label: &str, ok: bool) {
        if of_tool(label) == Phase::Compiling {
            if let Some((_, end @ None)) = &mut self.run {
                *end = Some(ok);
            }
        }
    }

    /// The turn's verdict, once: `None` when it edited nothing or the
    /// project's check spoke. The evidence is cleared for the next turn.
    pub(crate) fn settle(&mut self) -> Option<String> {
        let p = std::mem::take(self);
        if !p.edited || p.checked {
            return None;
        }
        Some(match p.run {
            Some((cmd, Some(true))) => {
                format!("\u{2713} checked: `{cmd}` passed after the last edit")
            }
            Some((cmd, Some(false))) => {
                format!("\u{2717} failing: `{cmd}` failed after the last edit")
            }
            Some((cmd, None)) => format!("? `{cmd}` was still running when the turn ended"),
            None => "! not checked: nothing was built or tested after the last edit".into(),
        })
    }
}

/// The command a call runs: Claude Code's `Bash` sends `{"command": …}`,
/// crew's `sys:run` `{"cmd": …}`.
fn command_of(args: &str) -> String {
    let v = serde_json::from_str::<serde_json::Value>(args).ok();
    let field = |k: &str| v.as_ref()?.get(k)?.as_str().map(str::to_string);
    field("command")
        .or_else(|| field("cmd"))
        .unwrap_or_default()
}

fn verifies(command: &str) -> bool {
    command
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| VERIFIES.contains(&w.to_ascii_lowercase().as_str()))
}

fn clip(command: &str) -> String {
    let one: String = command.split_whitespace().collect::<Vec<_>>().join(" ");
    match one.chars().count() > CMD_MAX {
        true => format!(
            "{}\u{2026}",
            one.chars().take(CMD_MAX - 1).collect::<String>()
        ),
        false => one,
    }
}

#[cfg(test)]
#[path = "chatproof_tests.rs"]
mod tests;
