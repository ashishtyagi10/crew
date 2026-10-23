//! Tool hooks: commands of the user's own that run before and after an agent's tool calls —
//! Claude Code's PreToolUse/PostToolUse, the contract Codex and Kiro hooks keep too. A pre
//! hook can stop a call (exit 2, or `{"decision": "block"}` on stdout) and the agent reads
//! why as the call's result; a post hook's stdout lands under the result, so a formatter or a
//! linter can answer back. Both read the call as one JSON line on stdin.
//!
//! Read from ONE file, `<config>/crew/hooks.json`, and never from the project. A hook runs a
//! command, and a project file is a file any cloned repo can ship: opening someone's repo must
//! never be a way to run their commands here. (`.crew/check` may live in the project because
//! crew believes only a build tool there — see `checkcmd`; a hook is any command at all.)
//!
//! Read on every call rather than cached. Absent, the common case, it costs one failed open;
//! present, it is a few lines of JSON. In exchange an edit lands on the very next call, the
//! way `mcp.json` and integrations reload without a restart.
use std::path::{Path, PathBuf};
use std::time::Duration;

#[path = "hookfile.rs"]
mod file;
#[path = "hookrun.rs"]
mod run;
pub(crate) use file::Source;
use file::{File, OnFailure, Rule};
use run::Ran;

/// How long one hook may run. It stands between the agent and every call it matches, so a
/// slow hook is a slow agent; five seconds is plenty for a guard or a formatter, and one that
/// hangs is killed rather than waited on.
pub(crate) const DEADLINE: Duration = Duration::from_secs(5);

/// Chars of a hook's answer the agent is shown: room for a linter's findings, not so much
/// that a chatty hook pushes the tool's own result out of the clip on what the agent reads.
const CAP: usize = 2_000;

/// The hooks one call runs under.
pub(crate) struct Hooks {
    /// `Err` when the file is there and unreadable: said on every call, since the other
    /// choice is a guard that looks on and is not.
    file: Result<File, String>,
    deadline: Duration,
    cwd: Option<PathBuf>,
}

impl Hooks {
    /// Whether `tool` (`server:name`) may run: `Err` carries the reason the agent is told.
    pub(crate) fn pre(&self, tool: &str, args: &str) -> Result<(), String> {
        let file = self.file.as_ref().map_err(|e| {
            format!("blocked by hook: {e} — fix the file, or set CREW_HOOKS=0 to turn hooks off")
        })?;
        for rule in file.pre_tool.iter().filter(|r| matches(&r.pattern, tool)) {
            match self.run(rule, "pre_tool", tool, args, None) {
                Ran::Exited(code, out, err) => {
                    if let Some(why) = blocked(code, &out, &err) {
                        return Err(format!("blocked by hook: {why}"));
                    }
                    // Only exit 2 says no (Claude Code's rule); anything else is a hook
                    // having trouble, which the person may want to know and the agent cannot use.
                    if code != 0 {
                        let why = err.lines().next().unwrap_or("").trim();
                        log(format!(
                            "hook `{}` on {tool} exited {code}, let through: {why}",
                            rule.cmd
                        ));
                    }
                }
                Ran::Failed(why) if rule.on_failure == OnFailure::Block => {
                    return Err(format!(
                        "blocked by hook: `{}` {why}, and it is set to block when it cannot run",
                        rule.cmd
                    ));
                }
                Ran::Failed(why) => {
                    log(format!("hook `{}` on {tool} {why}, let through", rule.cmd))
                }
            }
        }
        Ok(())
    }

    /// `result` with what each matching post hook printed under it. Never fails the call: the
    /// tool already ran, and a hook that broke is a note in the LOG, not the agent's problem.
    pub(crate) fn post(&self, tool: &str, args: &str, result: String) -> String {
        let Ok(file) = &self.file else { return result };
        let mut blocks = String::new();
        for rule in file.post_tool.iter().filter(|r| matches(&r.pattern, tool)) {
            // A linter exits 1 when it has something to say, so stdout counts whatever the code.
            match self.run(rule, "post_tool", tool, args, Some(&result)) {
                Ran::Exited(_, out, _) if !out.trim().is_empty() => {
                    blocks.push_str(&format!("\n\nhook: {}", cap(out.trim())));
                }
                Ran::Exited(..) => {}
                Ran::Failed(why) => log(format!("hook `{}` after {tool} {why}", rule.cmd)),
            }
        }
        result + &blocks
    }

    fn run(&self, rule: &Rule, event: &str, tool: &str, args: &str, result: Option<&str>) -> Ran {
        let cwd = self
            .cwd
            .clone()
            .or_else(|| crate::broker::gitmsg::project_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let line = stdin_line(event, tool, args, &cwd, result);
        run::run(&rule.cmd, &cwd, &line, self.deadline)
    }
}

/// An exact `server:tool`, every tool of a server (`server:*`), or everything (`*`).
fn matches(pattern: &str, tool: &str) -> bool {
    pattern == "*"
        || pattern == tool
        || pattern
            .strip_suffix(":*")
            .is_some_and(|server| tool.split_once(':').is_some_and(|(s, _)| s == server))
}

/// Why a hook that ran said no, or `None` when it let the call through.
fn blocked(code: i32, out: &str, err: &str) -> Option<String> {
    let said = serde_json::from_str::<serde_json::Value>(out.trim()).ok();
    let field = |k: &str| said.as_ref()?.get(k)?.as_str().map(str::trim);
    if code != 2 && field("decision") != Some("block") {
        return None;
    }
    let why = field("reason").filter(|r| !r.is_empty());
    let why = why.or(Some(err.trim()).filter(|e| !e.is_empty()));
    Some(cap(why.unwrap_or("a hook said no and gave no reason")))
}

/// The one JSON line a hook reads: what is called, with what, where — and after, what it said.
fn stdin_line(event: &str, tool: &str, args: &str, cwd: &Path, result: Option<&str>) -> String {
    // Arguments the model wrote as broken JSON still reach the hook, as the text they were.
    let args = serde_json::from_str(args).unwrap_or_else(|_| serde_json::Value::from(args));
    let cwd = cwd.to_string_lossy();
    let mut v = serde_json::json!({"event": event, "tool": tool, "args": args, "cwd": cwd});
    if let Some(r) = result {
        v["result"] = r.into();
    }
    format!("{v}\n")
}

fn cap(s: &str) -> String {
    match s.char_indices().nth(CAP) {
        Some((i, _)) => format!("{}\u{2026} (cut at {CAP} chars)", &s[..i]),
        None => s.to_string(),
    }
}

/// The host's activity LOG, not the transcript: silent where no host listens, as in tests.
fn log(message: String) {
    crate::broker::hostnote::status(true, message);
}

#[cfg(all(test, unix))]
#[path = "hooks_tests.rs"]
mod tests;

#[cfg(all(test, unix))]
#[path = "hookspost_tests.rs"]
mod post_tests;
