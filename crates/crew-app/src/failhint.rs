//! The one-time nudge after a command fails: how to hand it to agent smith.
//!
//! Shown by `poll` the moment a command returns the pane to its prompt having
//! failed, whatever its runtime (the system banner waits for a long one) — once
//! per failure, since a command finishes once — and never for a pane in the
//! background, where the key would act on the wrong pane.

/// The status line for a command that ended with `exit`, or `None` when it
/// did not fail (or the shell said nothing).
pub(crate) fn text(exit: Option<i32>) -> Option<String> {
    let code = exit.filter(|c| *c != 0)?;
    let key = crate::chordglyph::prose(crate::fixwithsmith::KEY);
    Some(format!(
        "failed (exit {code}) \u{2014} {key} or click the mark: fix with smith"
    ))
}

/// The hint for pane `t` as `poll` finds it back at its prompt: only when
/// `focused`, and only when the last command it ran failed.
pub(crate) fn seen(t: &crate::pane::TermPane, focused: bool) -> Option<String> {
    text(t.spans.last_exit()).filter(|_| focused && t.cmd.is_none())
}

#[cfg(test)]
#[path = "failhint_tests.rs"]
mod tests;
