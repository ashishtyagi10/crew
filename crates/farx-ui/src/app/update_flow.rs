//! In-TUI update flow: the `/update` slash command kicks off a background
//! GitHub release check whose result we poll each tick. The main loop calls
//! `complete_install` after running the blocking installer.

use crate::components::update_modal::UpdateState;

use super::App;

impl App {
    /// Called when the background update check finds a newer version.
    pub fn set_update_available(&mut self, version: String) {
        self.update_available = Some(version);
    }

    /// Called when the background updater has finished applying an update.
    pub fn set_update_applied(&mut self, version: String) {
        self.feedback
            .success(format!("Updated to v{version} — restart farx to use it"));
        self.update_available = None;
    }

    /// Kick off the `/update` flow: spawn the GitHub release check on a
    /// background thread and store the receiver. `poll_update_check()` picks
    /// the result up on a subsequent tick.
    pub(super) fn start_update_check(&mut self) {
        if self.update_state.is_some() {
            self.feedback
                .info("Update check already in progress".to_string());
            return;
        }
        self.feedback.info("Checking for updates…".to_string());
        let rx = farx_core::update::check_and_auto_update_async();
        self.update_state = Some(UpdateState::Checking { rx });
    }

    /// Drain the update-check channel without blocking. Transitions the
    /// `Checking` state into `Confirm` / `Failed` / cleared once the
    /// background thread produces a result.
    pub(super) fn poll_update_check(&mut self) {
        let Some(UpdateState::Checking { rx }) = self.update_state.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(status) => {
                use farx_core::update::UpdateStatus;
                let current = env!("CARGO_PKG_VERSION").to_string();
                match status {
                    UpdateStatus::Available(latest) => {
                        self.update_state = Some(UpdateState::Confirm { current, latest });
                    }
                    UpdateStatus::Updated(version) => {
                        self.update_state = Some(UpdateState::Done { version });
                    }
                    UpdateStatus::UpToDate => {
                        self.update_state = None;
                        self.feedback
                            .info(format!("Already on the latest version (v{})", current));
                    }
                    UpdateStatus::Failed(message) => {
                        self.update_state = Some(UpdateState::Failed { message });
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.update_state = Some(UpdateState::Failed {
                    message: "Update check ended without a result".to_string(),
                });
            }
        }
    }

    /// Called by the main loop after running `perform_update()`.
    /// `result` is `Ok(installed_version)` on success or `Err(message)` on
    /// failure.
    pub fn complete_install(&mut self, result: Result<String, String>) {
        self.update_state = Some(match result {
            Ok(version) => UpdateState::Done { version },
            Err(message) => UpdateState::Failed { message },
        });
    }
}
