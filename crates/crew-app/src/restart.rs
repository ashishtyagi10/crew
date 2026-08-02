//! Relaunch crew as a fresh detached process and exit this one — how `/update`
//! rides into a binary it just installed (the fresh process also re-reads
//! `config.toml`, so external config edits take effect too). Not a slash
//! command of its own since `/restart` merged into `/update`.
use crate::app::CrewApp;
use crate::detach;

impl CrewApp {
    /// Spawn a fresh detached crew and ask the app to exit. Returns `true`
    /// (exit) on success; if the spawn fails, this instance keeps running.
    pub(crate) fn restart_crew(&mut self) -> bool {
        match detach::spawn_detached_copy() {
            Ok(_) => true,
            Err(e) => {
                self.set_status(format!("restart failed: {e}"));
                false
            }
        }
    }
}
