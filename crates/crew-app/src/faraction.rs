//! App-side execution of the `FarAction`s a Far pane produces: from a live
//! key press (`farpane::keys::reduce`, routed via `keys.rs`) or from a
//! background remote op landing this tick (`FarPane::poll_ops`, routed via
//! `poll.rs`) — e.g. a finished `rclone` download opening its temp file.
//! Both paths share this one match so the behaviour (close the pane, open
//! the help overlay, show a file in the viewer or `$EDITOR`, hand a
//! downloaded remote temp file to the OS default app, flash a status) never
//! drifts between the two call sites.
use crate::app::CrewApp;
use crate::farpane::FarAction;

impl CrewApp {
    /// Execute a `FarAction` from the Far pane at index `focused`.
    pub(crate) fn apply_far_action(&mut self, action: FarAction, focused: usize) {
        match action {
            FarAction::Close => {
                self.close_pane(focused);
            }
            FarAction::Help => self.help_open = true,
            FarAction::Open(path) => {
                // `that_detached`, never `that`: the blocking form waits for
                // the opener to exit, and this runs on the winit thread, so
                // every pane would freeze until the user closed whatever
                // opened. On Windows an unassociated path raises the "How do
                // you want to open this file?" modal, which never returns on
                // a headless runner — that is what hung CI for 5.5 hours.
                let _ = open::that_detached(path);
            }
            FarAction::View(path) => self.open_view(&path.to_string_lossy()),
            FarAction::Edit(path) => self.edit_in_pane(&path.to_string_lossy()),
            FarAction::Status(msg) => self.set_status(&msg),
        }
    }
}

#[cfg(test)]
#[path = "faraction_tests.rs"]
mod tests;
