//! `/todo reminders [on|off]`: the sync switched, and the switch kept for the
//! next launch (`handler` starts it then).
use crate::app::CrewApp;

impl CrewApp {
    pub(crate) fn todo_reminders(&mut self, on: bool) {
        if !cfg!(target_os = "macos") {
            return self.set_status(super::MAC_ONLY);
        }
        self.config.todo_reminders = on;
        self.config.save();
        if on {
            super::start(self.applog.sender());
            self.set_status(
                "Reminders: /todo kept in step with all your reminders \u{b7} macOS asks once whether crew may",
            );
        } else {
            super::stop();
            self.set_status("Reminders: sync off \u{b7} your reminders stay as they are");
        }
    }
}
