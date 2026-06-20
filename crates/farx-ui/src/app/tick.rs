//! Per-tick application step: drains feedback timers, polls all background
//! channels (AI, file watcher, terminals, update check, progress), reloads
//! `follow`-mode viewers, and debounces typeahead suggestions.

use super::App;

impl App {
    /// Advance application state by one tick.
    pub fn tick(&mut self) {
        self.tick_count += 1;
        self.feedback.tick();
        self.check_ai_response();
        self.check_suggestion_response();
        self.check_fs_changes();
        self.poll_terminals();
        self.poll_update_check();

        if let Some(ref mut progress) = self.progress {
            if progress.poll() {
                let error = progress.error.clone();
                let files_done = progress.files_done;
                let op = progress.operation.clone();
                self.progress = None;
                if let Some(err) = error {
                    self.feedback.error(format!("{} failed: {}", op, err));
                } else {
                    self.feedback
                        .success(format!("{} complete: {} file(s)", op, files_done));
                }
                self.refresh_both_panels();
            }
        }

        if self.tick_count.is_multiple_of(4) {
            if let Some(ref mut viewer) = self.viewer {
                viewer.reload_if_follow();
            }
        }

        if !self.command_line.input.is_empty()
            && !self.command_line.suggestion_pending
            && self.command_line.suggestion.is_none()
            && self.command_line.last_typed_tick > 0
            && self.tick_count - self.command_line.last_typed_tick >= 3
        {
            self.request_suggestion();
        }
    }
}
