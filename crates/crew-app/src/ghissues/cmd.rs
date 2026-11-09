//! `/todo issues [on|off]`: the GitHub sync switched, and the switch kept.
use crate::app::CrewApp;

impl CrewApp {
    pub(crate) fn todo_issues(&mut self, on: bool) {
        super::set_off(!on);
        if on {
            super::start(self.applog.sender());
            self.set_status(
                "GitHub issues: every @project todo in a GitHub checkout is filed and kept in step",
            );
        } else {
            super::stop();
            self.set_status("GitHub issues: off \u{b7} issues already filed stay as they are");
        }
    }
}
