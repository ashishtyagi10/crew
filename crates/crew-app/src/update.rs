//! Background self-update with progress in the left-nav UPDATE card. `/update`
//! starts a worker thread that checks GitHub, downloads the latest release over
//! the running binary, and streams stage updates back to the UI — no separate
//! shell pane. Once the install lands, a loud (manual) run restarts Crew into
//! the new build after a short "restarting…" beat; only the SILENT background
//! check (see `autoupdate`) parks its install for a later `/update`, so a
//! session is never interrupted by anything the user didn't type.
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
/// How long the "updated vX / restarting…" card shows before a loud run's
/// install actually restarts Crew — long enough to read, short enough that
/// `/update` feels like one motion.
const RESTART_DELAY: Duration = Duration::from_secs(2);

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
                // Installed over the running binary. A loud run restarts into
                // it when this deadline elapses (`poll_update` raises the
                // restart tick); a silent install clears now and waits parked.
                self.deadline = Some(if self.silent {
                    now
                } else {
                    now + RESTART_DELAY
                });
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
    /// Start the merged update-and-restart (the `/update` command). Returns
    /// `true` when the app should exit because a restart was spawned.
    ///
    /// An install already parked (by the silent background check, or by a
    /// loud run whose restart failed) is applied immediately — restart, no
    /// network round-trip. Otherwise: a silent run already in flight is taken
    /// over — upgraded to loud — rather than refused, since a manual `/update`
    /// means the user now wants to watch it (and, now, ride it into the new
    /// build); a loud run already animating is refused as before, so a double
    /// `/update` doesn't spawn two workers.
    pub(crate) fn start_update(&mut self) -> bool {
        match update_cmd_action(
            self.parked_update.is_some(),
            self.update.as_ref().map(|u| (u.silent, u.animating())),
        ) {
            UpdateCmdAction::RestartParked => return self.restart_crew(),
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
        false
    }

    /// Start the quiet background update check (30 s after launch, then every
    /// 6 h — see [`crate::autoupdate`]). No status message, no redraw: unlike
    /// `/update` this must be invisible until something worth showing happens
    /// (a loud takeover via `start_update`, or a parked install). A no-op if a
    /// run is already active in either mode, or an install is already parked
    /// awaiting the next `/update` — nothing to gain checking again until then.
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
    /// `CrewApp::parked_update`; when a LOUD run's `Done` beat elapses, the
    /// tick asks the caller to restart into the new build. The park is not
    /// redundant with the restart: it is the persistent nav-legend reminder
    /// (`restartnote`) if the restart spawn fails, and the whole story for a
    /// silent install waiting on the next `/update`.
    pub(crate) fn poll_update(&mut self, now: Instant) -> UpdateTick {
        let mut tick = UpdateTick::default();
        let mut clear = false;
        let mut restart = false;
        if let Some(u) = self.update.as_mut() {
            tick.redraw = u.drain(now);
            // A silent background run has no UPDATE card to animate — driving
            // `tick.redraw` off its spinner ticks would repaint the whole
            // window at ~10 fps for nothing visible. Loud runs still animate.
            if !u.silent && u.animating() && u.tick_anim() {
                tick.redraw = true;
            }
            if let Stage::Done(v) = &u.stage {
                // Park on first install, and re-park (with a fresh stamp so
                // the blink pulse re-fires) whenever a *different* version
                // lands — e.g. a manual `/update` after an auto-parked one.
                // A repeat Done for the same version is a no-op: no stamp
                // refresh, no re-triggered blink nag.
                let should_park = match &self.parked_update {
                    None => true,
                    Some((pv, _)) => pv != v,
                };
                if should_park {
                    self.parked_update = Some((v.clone(), crate::anim::now_ms()));
                }
            }
            clear = u.clear_due(now);
            restart = clear && !u.silent && matches!(u.stage, Stage::Done(_));
        }
        if clear {
            self.update = None;
            tick.redraw = true;
            tick.restart = restart;
        }
        tick
    }
}

/// What `/update` should do, given whether an install is already parked and
/// the in-flight run's `(silent, animating)` state. Pure so the dispatch
/// order — parked install wins, then takeover, then refusal — is testable
/// without spawning a worker thread or a detached process.
#[derive(Debug, PartialEq)]
pub(crate) enum UpdateCmdAction {
    /// A newer binary is already installed: restart into it now.
    RestartParked,
    /// A silent background run is in flight: watch it (and restart on install).
    TakeoverSilent,
    /// A loud run is already animating: refuse the duplicate.
    AlreadyRunning,
    /// Nothing in flight: spawn the worker.
    Spawn,
}

pub(crate) fn update_cmd_action(parked: bool, run: Option<(bool, bool)>) -> UpdateCmdAction {
    if parked {
        return UpdateCmdAction::RestartParked;
    }
    match run {
        Some((true, _)) => UpdateCmdAction::TakeoverSilent,
        Some((false, true)) => UpdateCmdAction::AlreadyRunning,
        _ => UpdateCmdAction::Spawn,
    }
}

/// What one [`CrewApp::poll_update`] tick wants the caller to do.
#[derive(Default)]
pub(crate) struct UpdateTick {
    pub(crate) redraw: bool,
    /// A loud run's install has finished its "restarting…" beat: relaunch
    /// detached and exit this process.
    pub(crate) restart: bool,
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
