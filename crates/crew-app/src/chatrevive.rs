//! A broker that stops is said out loud and started again.
//!
//! The broker is a child process, and a child process can die: a panic, an
//! out-of-memory kill, a signal. Its output simply ends (`BROKER_ENDED`).
//! Until 0.25.29 nothing listened for that: no event ever came again, so the
//! task it was running showed as running forever, Esc and `/stop` wrote to a
//! dead pipe, and queued messages waited for an answer that could not come.
//! Now the pane says what was lost, starts the broker again — the mode the
//! user chose goes with it — and says how to send the message again. A broker
//! that keeps dying is left stopped rather than restarted in a loop.
use crew_plugin::{ApprovalMode, PluginCommand};

use crate::chat::ChatPane;
use crate::chatevents::HostAction;

/// Restarts allowed inside [`WINDOW_MS`] before the pane stops trying.
const MAX_RESTARTS: usize = 3;
const WINDOW_MS: u64 = 5 * 60 * 1000;

impl ChatPane {
    /// The broker's output ended (or it reported an error it cannot recover
    /// from): settle every piece of busy state it owned, say what did not
    /// finish, and start it again.
    pub(crate) fn broker_lost(&mut self, why: &str) -> HostAction {
        let what = match why == crew_plugin::BROKER_ENDED {
            true => "agent smith's process stopped: it exited, crashed or was killed".to_string(),
            false => format!("agent smith's process stopped: {why}"),
        };
        self.restart_broker(&what, true)
    }

    /// Settle the busy state a broker owned, say `what` happened and what did
    /// not finish, and start the broker again — `resend` adds how to send the
    /// last message again (not after an Esc: stopping was the point).
    pub(crate) fn restart_broker(&mut self, what: &str, resend: bool) -> HostAction {
        let lost = self.running_tasks.clone();
        let busy = self.is_busy();
        self.fold_swarm();
        self.flush_active_hops();
        self.reset_broker_state();
        (self.connected, self.awaiting) = (false, false);
        self.watch = Default::default();
        let unfinished = match (lost.as_slice(), busy) {
            ([], false) => String::new(),
            ([], true) => " What it was doing did not finish.".to_string(),
            (ids, _) => {
                let ids: Vec<String> = ids.iter().map(|i| format!("#{i}")).collect();
                let noun = if ids.len() == 1 { "Task" } else { "Tasks" };
                format!(" {noun} {} did not finish.", crate::wording::series(&ids))
            }
        };
        let now = crate::anim::now_ms();
        self.restarts.retain(|&t| now.saturating_sub(t) < WINDOW_MS);
        if self.restarts.len() >= MAX_RESTARTS {
            // Where THIS platform keeps it: the path was written out as the
            // Mac's, which is wrong on Windows and Linux.
            let log = crate::crashlog::log_path()
                .map_or_else(|| "crash.log".into(), |p| crate::cwdshow::display(&p));
            self.push_note(format!(
                "\u{2717} {what} \u{2014} {MAX_RESTARTS} times in five minutes, so it is left \
                 stopped.{unfinished} The last crash is in {log}; close this pane and open \
                 /smith to start fresh."
            ));
            return status("agent smith keeps stopping \u{2014} left stopped");
        }
        self.restarts.push(now);
        if let Err(e) = self.plugin.restart() {
            self.push_note(format!(
                "\u{2717} {what}, and starting it again failed: {e}.{unfinished} Close this \
                 pane and open /smith to try again."
            ));
            return status("agent smith stopped and could not be restarted");
        }
        let _ = self.plugin.send(&PluginCommand::Hello { v: 1 });
        if self.approval_mode != ApprovalMode::Auto {
            let mode = self.approval_mode;
            let _ = self.plugin.send(&PluginCommand::Mode { approval: mode });
        }
        let again = match resend {
            true => " Press \u{2191} then Enter to send your last message again.",
            false => "",
        };
        self.push_note(format!(
            "\u{2717} {what}. It was started again.{unfinished}{again}"
        ));
        status("agent smith restarted after its process stopped")
    }
}

fn status(message: &str) -> HostAction {
    HostAction::Status {
        error: true,
        message: message.to_string(),
    }
}

#[cfg(test)]
#[path = "chatrevive_tests.rs"]
mod tests;
