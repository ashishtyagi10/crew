//! Background self-update with progress in the left-nav UPDATE card. `/update`
//! starts a worker thread that checks GitHub, downloads the latest release over
//! the running binary, and streams stage updates back to the UI — no separate
//! shell pane. The new binary applies on the next launch; Crew does NOT restart
//! itself, so an in-flight session is never interrupted.
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::app::CrewApp;

/// Spinner frames cycled on the UPDATE card while a stage is in flight.
pub(crate) const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
/// Poll ticks per spinner frame (~62 Hz loop → ~10 fps).
const SPINNER_DIV: u64 = 6;
/// How long a terminal card (installed / up-to-date / failed) lingers before
/// auto-dismiss.
const NOTE_TTL: Duration = Duration::from_secs(5);

/// A stage message streamed from the worker thread to the UI.
pub(crate) enum UpdateMsg {
    Checking,
    Downloading(String),
    Installed(String),
    UpToDate(String),
    Failed(String),
}

/// Where the update is right now, mirrored from the latest [`UpdateMsg`].
pub(crate) enum Stage {
    Checking,
    Downloading(String),
    Done(String),
    Note(String),
}

/// Live update state held on `CrewApp` while `/update` runs.
pub(crate) struct UpdateState {
    rx: Receiver<UpdateMsg>,
    pub(crate) stage: Stage,
    pub(crate) spinner: usize,
    frame: u64,
    /// Restart-at (after Done) or clear-at (after a terminal note).
    deadline: Option<Instant>,
}

impl UpdateState {
    fn new(rx: Receiver<UpdateMsg>) -> Self {
        Self {
            rx,
            stage: Stage::Checking,
            spinner: 0,
            frame: 0,
            deadline: None,
        }
    }

    /// Drain pending worker messages into `stage`. Returns true if it changed.
    /// `try_recv` ending in either `Empty` or `Disconnected` stops the drain.
    fn drain(&mut self, now: Instant) -> bool {
        let mut changed = false;
        while let Ok(msg) = self.rx.try_recv() {
            self.apply(msg, now);
            changed = true;
        }
        changed
    }

    fn apply(&mut self, msg: UpdateMsg, now: Instant) {
        self.stage = match msg {
            UpdateMsg::Checking => Stage::Checking,
            UpdateMsg::Downloading(v) => Stage::Downloading(v),
            UpdateMsg::Installed(v) => {
                // Installed over the running binary; it applies on the next launch.
                // Linger the card briefly, then clear — no auto-restart.
                self.deadline = Some(now + NOTE_TTL);
                Stage::Done(v)
            }
            UpdateMsg::UpToDate(v) => {
                self.deadline = Some(now + NOTE_TTL);
                Stage::Note(format!("already up to date (v{v})"))
            }
            UpdateMsg::Failed(e) => {
                self.deadline = Some(now + NOTE_TTL);
                Stage::Note(format!("update failed: {e}"))
            }
        };
    }

    /// True while a network/disk stage is in flight (so the spinner animates).
    pub(crate) fn animating(&self) -> bool {
        matches!(self.stage, Stage::Checking | Stage::Downloading(_))
    }

    /// Advance the spinner on a throttle; returns true when its frame changed.
    fn tick_anim(&mut self) -> bool {
        self.frame = self.frame.wrapping_add(1);
        if self.frame.is_multiple_of(SPINNER_DIV) {
            self.spinner = self.spinner.wrapping_add(1);
            return true;
        }
        false
    }

    /// A terminal card (installed / note) whose linger has elapsed → dismiss it.
    fn clear_due(&self, now: Instant) -> bool {
        matches!(self.stage, Stage::Done(_) | Stage::Note(_))
            && self.deadline.is_some_and(|d| now >= d)
    }

    /// Build a state parked at `stage` (no worker thread) — for card-render tests.
    #[cfg(test)]
    pub(crate) fn for_test(stage: Stage) -> Self {
        let (_tx, rx) = std::sync::mpsc::channel();
        Self {
            rx,
            stage,
            spinner: 0,
            frame: 0,
            deadline: None,
        }
    }
}

impl CrewApp {
    /// Start the background self-update (the `/update` command). A no-op while one
    /// is already running, so a double `/update` doesn't spawn two workers.
    pub(crate) fn start_update(&mut self) {
        if self.update.as_ref().is_some_and(UpdateState::animating) {
            self.set_status("update already in progress");
            return;
        }
        self.update = Some(UpdateState::new(crate::updatefetch::spawn_worker()));
        self.set_status("checking for updates…");
        self.redraw();
    }

    /// Drive the active update each poll tick. Streams stage changes into the
    /// UPDATE card and dismisses it once a terminal card's linger elapses.
    pub(crate) fn poll_update(&mut self, now: Instant) -> UpdateTick {
        let mut tick = UpdateTick::default();
        let mut clear = false;
        if let Some(u) = self.update.as_mut() {
            tick.redraw = u.drain(now);
            if u.animating() && u.tick_anim() {
                tick.redraw = true;
            }
            clear = u.clear_due(now);
        }
        if clear {
            self.update = None;
            tick.redraw = true;
        }
        tick
    }
}

/// What one [`CrewApp::poll_update`] tick wants the caller to do.
#[derive(Default)]
pub(crate) struct UpdateTick {
    pub(crate) redraw: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::CrewApp;

    #[test]
    #[allow(clippy::field_reassign_with_default)] // test fixture: inject update state
    fn install_parks_then_clears_without_restarting() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = CrewApp::default();
        app.update = Some(UpdateState::new(rx));
        let now = Instant::now();
        tx.send(UpdateMsg::Installed("9.9.9".into())).unwrap();
        // First tick drains the install message and parks the card at "done".
        let tick = app.poll_update(now);
        assert!(tick.redraw);
        assert!(matches!(app.update.as_ref().unwrap().stage, Stage::Done(_)));
        // After the note lingers, the card auto-clears — the app is never asked to
        // restart (UpdateTick no longer carries a restart signal at all).
        app.poll_update(now + NOTE_TTL);
        assert!(app.update.is_none(), "card cleared, app keeps running");
    }
}
