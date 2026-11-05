//! Focus reporting (DECSET 1004, child module of `pty`). A program that turns
//! it on (neovim for `FocusGained`/`autoread`, tmux, vim's cursor shape, TUIs
//! that pause or dim) expects `CSI I` when its pane gains keyboard focus and
//! `CSI O` when it loses it. alacritty tracks the mode but leaves the writing
//! to the terminal; without this, crew never told anyone.
//!
//! The app does not hook the twenty places focus moves. It hands each pane its
//! focus once per tick and this diffs it against what the program was last
//! told, so a report is written exactly when the answer changes.
use std::io::Write;

use super::PtyTerm;

/// What to tell the program, given what it was last `told`, whether its pane
/// has focus `now`, and whether it has DECSET 1004 `enabled`. Returns the new
/// `told` and the bytes to write, if any.
///
/// While the mode is off nothing is remembered, so turning it on starts
/// fresh. The first tick with it on only latches (xterm sends nothing on
/// enable either; a program that just asked is in the foreground and assumes
/// focus), except that an unfocused pane says so at once: the program's
/// assumption is wrong and nothing else would correct it.
pub fn focus_report(
    told: Option<bool>,
    now: bool,
    enabled: bool,
) -> (Option<bool>, Option<&'static [u8]>) {
    if !enabled {
        return (None, None);
    }
    let bytes: &'static [u8] = if now { b"\x1b[I" } else { b"\x1b[O" };
    match told {
        Some(was) if was == now => (told, None),
        None if now => (Some(true), None),
        _ => (Some(now), Some(bytes)),
    }
}

impl PtyTerm {
    /// Tell the program its pane's keyboard focus when DECSET 1004 is on and
    /// the answer changed since last time. Returns whether anything was
    /// written. Best-effort like every pty write: a dead child's pipe error
    /// surfaces through its own exit path.
    pub fn report_focus(&mut self, focused: bool) -> bool {
        let enabled = self.core.input_modes().focus_events;
        let (told, bytes) = focus_report(self.focus_told, focused, enabled);
        self.focus_told = told;
        let Some(bytes) = bytes else {
            return false;
        };
        let _ = self.input.write_all(bytes).and_then(|_| self.input.flush());
        true
    }
}

#[cfg(test)]
#[path = "focusreport_tests.rs"]
mod tests;
