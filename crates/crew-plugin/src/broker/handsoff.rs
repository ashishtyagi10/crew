//! A review reads; it never edits.
//!
//! Measured live (2026-11-04, loop 39 S8): "review my diff" in a scratch repo
//! whose `lib.rs` changed `add(a, b)` to `add(a, b, c)` reached opencode,
//! whose CLI edits in place, and it came back "Review complete" having
//! rewritten the two callers in `app.rs` and `cli.rs`. A reviewer that fixes
//! what it was asked to judge takes the decision away from the person who
//! asked. Each CLI crew drives has its own read-only mode, and a review call
//! runs in it: Claude Code's plan mode, opencode's `plan` agent, codex's
//! read-only sandbox. Each was checked against its CLI by asking it to append
//! a line to a file: the file stayed as it was.

/// Claude Code's read-only mode: it reads and reasons, and refuses edits.
pub(crate) const CLAUDE: [&str; 2] = ["--permission-mode", "plan"];

/// The flags that keep `program` (a CLI crew drives, by its file name) from
/// writing, placed after its subcommand, which is where both CLIs read them.
/// Empty for a program crew does not know: a manifest agent's own command.
fn flags(program: &str) -> &'static [&'static str] {
    let name = std::path::Path::new(program)
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or(program);
    match name {
        "codex" => &["--sandbox", "read-only"],
        "opencode" => &["--agent", "plan"],
        _ => &[],
    }
}

/// Whether crew knows a read-only mode for `program`. A manifest agent's
/// own command has none crew can name, so its review is guarded instead
/// (`reviewguard`).
pub(crate) fn known(program: &str) -> bool {
    !flags(program).is_empty()
}

/// `args` (the call's argv, body already in place) with `program`'s
/// read-only flags after the first argument, its subcommand (`exec`, `run`).
pub(crate) fn cli_args(program: &str, mut args: Vec<String>) -> Vec<String> {
    let at = args.len().min(1);
    for (i, f) in flags(program).iter().enumerate() {
        args.insert(at + i, (*f).to_string());
    }
    args
}

/// One Claude Code call in plan mode, on a fresh session: a review must not
/// take the pane's session, nor leave it continuing in plan mode.
pub(crate) fn claude(
    a: &super::claudeagent::ClaudeAgent,
    body: &str,
    timeout: std::time::Duration,
) -> Result<String, String> {
    use super::adapter::Adapter;
    let me = super::claudeagent::ClaudeAgent {
        program: a.program.clone(),
        model: a.model.clone(),
        idle_floor: a.idle_floor,
        resume: Default::default(),
        read_only: true,
    };
    me.call(body, timeout)
}

#[cfg(test)]
#[path = "handsoff_tests.rs"]
mod tests;
