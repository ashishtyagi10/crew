//! Presses on crew's own docked buttons — the nav's RESTART card and a tile's
//! `[x]` / `[-]` — gathered in one place.
//!
//! All three share a rule: they must win over the focus/drag path in `events`,
//! or the click that closes a pane also focuses it and arms a text selection.
//! Grouping them keeps that ordering one decision instead of three, and keeps
//! `events.rs` (long before any of this) from growing another arm per button.
use crate::app::CrewApp;

/// What a press on a docked button asks the event loop to do.
pub(crate) enum Press {
    /// Nothing here: fall through to the focus/drag path.
    Miss,
    /// Handled (and already repainted): the click does nothing else.
    Done,
    /// Handled by riding a newly installed binary — this process is finished.
    Exit,
}

impl CrewApp {
    /// Test the docked buttons under the cursor, most-specific first, and act
    /// on the first hit.
    ///
    /// The nav's RESTART card leads: it sits in the sidebar, above every
    /// pane-row path, and it is the only one of the three whose outcome is to
    /// stop being this process.
    pub(crate) fn docked_press(&mut self) -> Press {
        if self.restart_btn_at_cursor() && self.restart_crew() {
            return Press::Exit;
        }
        if let Some(i) = self.close_btn_at_cursor() {
            self.close_pane(i);
        } else if let Some(i) = self.min_btn_at_cursor() {
            self.minimize_pane(i);
        } else {
            return Press::Miss;
        }
        self.redraw();
        Press::Done
    }
}
