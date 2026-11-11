//! Background self-update. A worker thread checks GitHub, downloads the latest
//! release over the running binary, and streams stage updates back — no
//! separate shell pane. The check runs by itself (see `autoupdate`), silently;
//! typing `/update` runs it now, with progress in the left-nav UPDATE card.
//! Either way the install PARKS: a note says a new version is ready, and crew
//! goes on running the old one until `/restart` is typed — a session is never
//! interrupted by anything the user didn't ask for.
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::app::CrewApp;

/// Spinner frames cycled on the UPDATE card while a stage is in flight.
pub(crate) const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
/// Poll ticks per spinner frame (~62 Hz loop → ~10 fps).
const SPINNER_DIV: u64 = 6;

/// The spinner glyph for `now_ms`, held on its first frame when motion is off.
/// One derivation for every spinner in the app, so a new one cannot be added
/// that ignores the setting.
pub(crate) fn spinner_frame(now_ms: u64) -> char {
    if crate::motion::level() == crate::motion::MotionLevel::Off {
        return SPINNER[0];
    }
    SPINNER[(now_ms / 100) as usize % SPINNER.len()]
}
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

/// Live update state held on `CrewApp` while a check runs.
pub(crate) struct UpdateState {
    rx: Receiver<UpdateMsg>,
    pub(crate) stage: Stage,
    pub(crate) spinner: usize,
    frame: u64,
    /// Restart-at (after Done) or clear-at (after a terminal note).
    deadline: Option<Instant>,
    /// Quiet background run: no UPDATE card, no status line, and a terminal
    /// note clears on the next tick instead of lingering. Set by
    /// `start_auto_update`; a manual `/update` upgrades an in-flight silent
    /// run to loud instead of refusing it.
    pub(crate) silent: bool,
}

impl UpdateState {
    /// Loud (default) construction — delegates to [`Self::new_with`].
    fn new(rx: Receiver<UpdateMsg>) -> Self {
        Self::new_with(rx, false)
    }

    pub(crate) fn new_with(rx: Receiver<UpdateMsg>, silent: bool) -> Self {
        Self {
            rx,
            stage: Stage::Checking,
            spinner: 0,
            frame: 0,
            deadline: None,
            silent,
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
        // Loud mode lingers a terminal card for `NOTE_TTL` before clearing; a
        // silent run clears within the same `poll_update` call — deadline == now,
        // so `clear_due` fires immediately after the drain. Nothing to show,
        // and (for `Installed`) the parked-update reminder captured in
        // `poll_update` carries the news instead of a lingering card.
        let clear_at = if self.silent { now } else { now + NOTE_TTL };
        self.stage = match msg {
            UpdateMsg::Checking => Stage::Checking,
            UpdateMsg::Downloading(v) => Stage::Downloading(v),
            UpdateMsg::Installed(v) => {
                // Installed over the running binary, and parked by
                // `poll_update` either way; a loud run's card lingers to say so.
                self.deadline = Some(clear_at);
                Stage::Done(v)
            }
            UpdateMsg::UpToDate(v) => {
                self.deadline = Some(clear_at);
                Stage::Note(format!("already up to date (v{v})"))
            }
            UpdateMsg::Failed(e) => {
                self.deadline = Some(clear_at);
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
            silent: false,
        }
    }
}

impl CrewApp {
    /// Check for a release NOW (the `/update` spelling) instead of waiting for
    /// the background check. It never restarts — that is `/restart`'s.
    ///
    /// An install already parked needs no network round-trip, only the
    /// reminder. Otherwise: a silent run already in flight is taken over —
    /// upgraded to loud — rather than refused, since a typed `/update` means
    /// the user now wants to watch it; a loud run already animating is
    /// refused, so a double `/update` doesn't spawn two workers.
    pub(crate) fn start_update(&mut self) {
        match update_cmd_action(
            self.parked_update.is_some(),
            self.update.as_ref().map(|u| (u.silent, u.animating())),
        ) {
            UpdateCmdAction::Parked => {
                let v = self.parked_update.as_ref().map_or("", |p| p.0.as_str());
                self.set_status(crate::restartnote::ready(v));
            }
            UpdateCmdAction::TakeoverSilent => {
                self.update.as_mut().expect("takeover implies a run").silent = false;
                self.set_status("checking for updates…");
            }
            UpdateCmdAction::AlreadyRunning => self.set_status("update already in progress"),
            UpdateCmdAction::Spawn => {
                let log = self.applog.sender();
                self.update = Some(UpdateState::new(crate::updatefetch::spawn_worker(log)));
                self.set_status("checking for updates…");
                self.redraw();
            }
        }
    }

    /// Start the quiet background update check (30 s after launch, then every
    /// half hour — see [`crate::autoupdate`]). No status message, no redraw:
    /// unlike `/update` this must be invisible until something worth showing
    /// happens (a loud takeover via `start_update`, or a parked install). A
    /// no-op if a run is already active in either mode, or an install is
    /// already parked awaiting `/restart` — it would only fetch that again.
    pub(crate) fn start_auto_update(&mut self) {
        if self.update.is_some() || self.parked_update.is_some() {
            return;
        }
        self.update = Some(UpdateState::new_with(
            crate::updatefetch::spawn_worker(self.applog.sender()),
            true,
        ));
    }

    /// Drive the active update each poll tick. Streams stage changes into the
    /// UPDATE card and dismisses it once a terminal card's linger elapses. Any
    /// run — silent or loud — that reaches `Installed` parks its version on
    /// `CrewApp::parked_update` and says so once (`note_update_ready`); the
    /// nav's RESTART card keeps saying it until `/restart`. Returns whether
    /// anything changed on screen.
    pub(crate) fn poll_update(&mut self, now: Instant) -> bool {
        let mut redraw = false;
        let mut clear = false;
        let mut ready = None;
        if let Some(u) = self.update.as_mut() {
            redraw = u.drain(now);
            // A silent background run has no UPDATE card to animate — driving
            // `tick.redraw` off its spinner ticks would repaint the whole
            // window at ~10 fps for nothing visible. Loud runs still animate.
            if !u.silent && u.animating() && u.tick_anim() {
                redraw = true;
            }
            if let Stage::Done(v) = &u.stage {
                // Park on first install, and re-park (with a fresh stamp and
                // a fresh note) whenever a *different* version lands. A
                // repeat Done for the same version is a no-op: no stamp
                // refresh, no second note.
                let should_park = match &self.parked_update {
                    None => true,
                    Some((pv, _)) => pv != v,
                };
                if should_park {
                    self.parked_update = Some((v.clone(), crate::anim::now_ms()));
                    ready = Some(v.clone());
                }
            }
            clear = u.clear_due(now);
        }
        if let Some(v) = ready {
            self.note_update_ready(&v);
            redraw = true;
        }
        if clear {
            self.update = None;
            redraw = true;
        }
        redraw
    }

    /// Say, once, that a new version is installed and waiting: a toast (the
    /// nav may be closed, and its RESTART card with it) and the status line.
    fn note_update_ready(&mut self, version: &str) {
        let note = crate::restartnote::ready(version);
        self.toasts
            .push(note.clone(), "update", false, crate::anim::now_ms());
        self.set_status(note);
    }
}

/// What `/update` should do, given whether an install is already parked and
/// the in-flight run's `(silent, animating)` state. Pure so the dispatch
/// order — parked install wins, then takeover, then refusal — is testable
/// without spawning a worker thread.
#[derive(Debug, PartialEq)]
pub(crate) enum UpdateCmdAction {
    /// A newer binary is already installed: say `/restart` applies it.
    Parked,
    /// A silent background run is in flight: watch it.
    TakeoverSilent,
    /// A loud run is already animating: refuse the duplicate.
    AlreadyRunning,
    /// Nothing in flight: spawn the worker.
    Spawn,
}

pub(crate) fn update_cmd_action(parked: bool, run: Option<(bool, bool)>) -> UpdateCmdAction {
    if parked {
        return UpdateCmdAction::Parked;
    }
    match run {
        Some((true, _)) => UpdateCmdAction::TakeoverSilent,
        Some((false, true)) => UpdateCmdAction::AlreadyRunning,
        _ => UpdateCmdAction::Spawn,
    }
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "updatepark_tests.rs"]
mod park_tests;
