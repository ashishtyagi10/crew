//! The pane's watchdog: a task is never left looking busy with nothing
//! behind it, and Esc always works.
//!
//! Three clocks, all on the animation clock (which does not run while the
//! machine sleeps, so a night with the lid shut is not a silence):
//!
//! * a broker that has been sending heartbeats (`PluginEvent::Alive`) and
//!   stops for [`WEDGED_MS`] has hung — it is restarted, and the pane says so;
//! * a task with nothing new for [`QUIET_MS`] gets a note saying what it is
//!   waiting on and that Esc stops it — again every [`QUIET_AGAIN_MS`];
//! * an Esc that has not ended the task within [`STOP_MS`] ends the broker
//!   instead: a model call or a tool deep in a worker cannot always be
//!   interrupted, but the process that holds it always can.
use crew_plugin::PluginEvent;

use crate::chat::ChatPane;
use crate::chatevents::HostAction;

const WEDGED_MS: u64 = 4 * crew_plugin::HEARTBEAT_MS;
const QUIET_MS: u64 = 90_000;
const QUIET_AGAIN_MS: u64 = 180_000;
const STOP_MS: u64 = 20_000;

/// The watchdog's clocks for one pane.
#[derive(Default)]
pub(crate) struct Watch {
    /// The last event of any kind, heartbeats included.
    last_event: u64,
    /// The last event that was not a heartbeat — the last sign of work.
    last_work: u64,
    /// This broker sends heartbeats, so their absence means something.
    beats: bool,
    /// When the quiet note was last pushed, for this quiet stretch.
    noted: Option<u64>,
    /// When Esc (or a typed `/stop`) was sent, while it has not taken.
    pub(crate) stop_asked: Option<u64>,
    /// When the turn running now began…
    turn_since: Option<u64>,
    /// …and, once it ended, how long it ran — taken by the app (`turndone`).
    pub(crate) turn_done: Option<u64>,
}

impl ChatPane {
    /// Every event from the broker, before anything else reads it.
    pub(crate) fn heard(&mut self, ev: &PluginEvent) {
        let now = crate::anim::now_ms();
        self.watch.last_event = now;
        match ev {
            PluginEvent::Alive {} => self.watch.beats = true,
            _ => (self.watch.last_work, self.watch.noted) = (now, None),
        }
    }

    /// Stamp a turn's start, and at its end how long it ran.
    pub(crate) fn track_turn(&mut self) {
        let now = crate::anim::now_ms();
        match (self.is_busy(), self.watch.turn_since) {
            (true, None) => self.watch.turn_since = Some(now),
            (false, Some(since)) => {
                self.watch.turn_since = None;
                self.watch.turn_done = Some(now.saturating_sub(since));
            }
            _ => {}
        }
    }

    /// Check the clocks; a restart's status line when one was needed.
    pub(crate) fn watchdog(&mut self) -> Option<HostAction> {
        self.watchdog_at(crate::anim::now_ms())
    }

    /// [`Self::watchdog`] at `now`, so a test can stand anywhere in time.
    fn watchdog_at(&mut self, now: u64) -> Option<HostAction> {
        let busy = self.is_busy();
        if !busy {
            // Quiet counts from the start of the next task, not from the
            // last event of the one before it.
            (self.watch.last_work, self.watch.stop_asked) = (now, None);
        }
        if !self.connected {
            return None;
        }
        if self.watch.beats && now.saturating_sub(self.watch.last_event) > WEDGED_MS {
            let what = "agent smith stopped answering: no sign of life for 20 seconds";
            return Some(self.restart_broker(what, true));
        }
        if self
            .watch
            .stop_asked
            .is_some_and(|t| now.saturating_sub(t) > STOP_MS)
        {
            self.watch.stop_asked = None;
            let what =
                "agent smith did not stop within 20 seconds of Esc, so its process was ended";
            return Some(self.restart_broker(what, false));
        }
        let quiet = now.saturating_sub(self.watch.last_work);
        let again = self
            .watch
            .noted
            .is_none_or(|t| now.saturating_sub(t) > QUIET_AGAIN_MS);
        if busy && quiet > QUIET_MS && again {
            self.watch.noted = Some(now);
            let note = format!(
                "still working \u{2014} nothing new for {}, {}. Esc stops it.",
                crate::runclock::ladder(quiet / 1000),
                self.waiting_on()
            );
            self.push_note(note);
        }
        None
    }

    /// What a quiet task is waiting on, in a few words.
    fn waiting_on(&self) -> String {
        match self.active.first() {
            Some(a) => match &a.tool {
                Some(tool) => format!("{} is waiting on {tool}", a.name),
                None => format!("{} is waiting for its model to reply", a.name),
            },
            None => "waiting on agent smith".to_string(),
        }
    }
}

#[cfg(test)]
#[path = "chatwatch_tests.rs"]
mod tests;
