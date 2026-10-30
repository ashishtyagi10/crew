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
//! same turn, its verdict is the one, and this stays quiet. A test run's own
//! count (`testtally`) goes into the verdict, and a run of no tests at all —
//! a filter that matched nothing exits 0 — is not a check.
use crew_hive::HiveEvent;
use crew_plugin::PluginEvent;

use crate::chatphase::{of_tool, Phase};
use crate::testtally::{tally, Tally};

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
    /// while it runs), with what its output counted.
    run: Option<(String, Option<bool>, Option<Tally>, u64)>,
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
                event:
                    HiveEvent::ToolResult {
                        label,
                        ok,
                        text,
                        ms,
                        ..
                    },
            } => self.result(label, *ok, text, *ms),
            PluginEvent::Message { text, .. } if text.starts_with("check: ") => self.checked = true,
            PluginEvent::Message { text, .. } => {
                let Some(card) = text.strip_prefix(crate::chatcard::TOOL_PREFIX) else {
                    return;
                };
                let first = card.lines().next().unwrap_or_default();
                let (label, rest) = first.split_once(' ').unwrap_or((first, ""));
                match (first.contains('\u{2713}'), first.contains('\u{2717}')) {
                    (false, false) => self.call(label, rest.trim()),
                    (ok, _) => self.result(label, ok, "", 0),
                }
            }
            _ => {}
        }
    }

    fn call(&mut self, label: &str, command: &str) {
        match of_tool(label) {
            Phase::Rewriting => (self.edited, self.run) = (true, None),
            Phase::Compiling if verifies(command) => {
                self.run = Some((clip(command), None, None, 0));
            }
            _ => {}
        }
    }

    fn result(&mut self, label: &str, ok: bool, out: &str, ms: u64) {
        if of_tool(label) == Phase::Compiling {
            if let Some((_, end @ None, counted, took)) = &mut self.run {
                (*end, *counted, *took) = (Some(ok), tally(out), ms);
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
            Some((cmd, Some(true), Some(t), _)) if t.ran() == 0 => {
                format!("! not checked: `{cmd}` ran no tests after the last edit")
            }
            Some((cmd, Some(ok), Some(t), ms)) => counted(&cmd, ok, &t, ms),
            Some((cmd, Some(true), ..)) => {
                format!("\u{2713} checked: `{cmd}` passed after the last edit")
            }
            Some((cmd, Some(false), ..)) => {
                format!("\u{2717} failing: `{cmd}` failed after the last edit")
            }
            Some((cmd, None, ..)) => format!("? `{cmd}` was still running when the turn ended"),
            None => "! not checked: nothing was built or tested after the last edit".into(),
        })
    }
}

/// The verdict on a run that counted its tests, the count straight after
/// the command so a folded receipt's cut keeps it: `— 42 tests passed in
/// 12s after the last edit`, `— 2 of 44 tests failed after the last edit:
/// name, name +1`.
fn counted(cmd: &str, ok: bool, t: &Tally, ms: u64) -> String {
    let tests = |n: u32| format!("{n} test{}", if n == 1 { "" } else { "s" });
    if ok && t.failed == 0 {
        let took = (ms >= 1000).then(|| format!(" in {}", crate::chatswarmspan::fmt_ms(ms)));
        let took = took.unwrap_or_default();
        let n = tests(t.ran());
        return format!("\u{2713} checked: `{cmd}` \u{2014} {n} passed{took} after the last edit");
    }
    let short = |n: &String| n.rsplit("::").next().unwrap_or(n).to_string();
    let named: Vec<String> = t.names.iter().take(2).map(short).collect();
    let names = match (named.is_empty(), t.names.len().saturating_sub(2)) {
        (true, _) => String::new(),
        (false, 0) => format!(": {}", named.join(", ")),
        (false, m) => format!(": {} +{m}", named.join(", ")),
    };
    let (f, n) = (t.failed, tests(t.ran()));
    format!("\u{2717} failing: `{cmd}` \u{2014} {f} of {n} failed after the last edit{names}")
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

#[cfg(test)]
#[path = "chatprooftally_tests.rs"]
mod tally_tests;
