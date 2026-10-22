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
                "Reminders: /todo and the crew list kept in step \u{b7} macOS asks once whether crew may",
            );
        } else {
            super::stop();
            self.set_status("Reminders: sync off \u{b7} the crew list stays as it is");
        }
    }
}
