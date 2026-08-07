//! Pushes light/dark scheme changes to terminal programs that asked for them
//! (DECSET 2031 — sniffed per pane by crew-term's `schemenotify`). CLIs
//! sample OSC 10/11 once at startup; without this, a mid-session theme flip
//! leaves a TUI on the old palette with only the contrast floor keeping it
//! readable. Opted-in programs (neovim's TUI and friends) get a
//! `CSI ? 997 ; Ps n` report the tick the active theme's darkness flips —
//! any switch path: an OS appearance flip under `auto`, `/theme`,
//! `Ctrl+Shift+L`, the Settings form.
use std::io::Write;

use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// Per-poll-tick: latch the active scheme and, when its darkness flipped,
    /// report it to every terminal pane that enabled DECSET 2031. The first
    /// tick only latches (startup is not a change — the program will query
    /// `CSI ? 996 n` itself if it cares). Rotations within one pool keep the
    /// darkness, so this stays quiet through them. Returns whether anything
    /// was written (callers don't currently need it, but the tests do).
    pub(crate) fn push_scheme_change(&mut self) -> bool {
        let dark = crew_theme::theme().dark;
        let first = self.scheme_pushed.is_none();
        if self.scheme_pushed == Some(dark) {
            return false;
        }
        self.scheme_pushed = Some(dark);
        if first {
            return false;
        }
        let mut wrote = false;
        for p in &mut self.panes {
            let PaneContent::Terminal(t) = &mut p.content else {
                continue;
            };
            if !t.pty.scheme_notify_enabled() {
                continue;
            }
            // Best-effort like every other pty write: a dead child's pipe
            // error surfaces through its own exit path, not here.
            let _ = t
                .input
                .write_all(crew_term::scheme_report(dark).as_bytes())
                .and_then(|_| t.input.flush());
            wrote = true;
        }
        wrote
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn first_tick_latches_without_reporting_then_flips_report() {
        let _g = crate::app::theme_test_guard();
        crew_theme::apply_selection(
            crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
            0,
        );
        let mut app = CrewApp::default();
        // First tick: latch only — startup must not spray reports.
        assert!(!app.push_scheme_change());
        assert_eq!(app.scheme_pushed, Some(true));
        // Same darkness again: quiet (a dark→dark rotation is not a change).
        assert!(!app.push_scheme_change());
        // Flip to light: the latch moves. (No panes here, so nothing is
        // written — pane-level formatting is covered by crew-term's
        // scheme_report tests; this test pins the latch protocol.)
        crew_theme::apply_selection(
            crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperLight),
            1,
        );
        app.push_scheme_change();
        assert_eq!(app.scheme_pushed, Some(false));
        crew_theme::apply_selection(
            crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
            2,
        );
    }
}
