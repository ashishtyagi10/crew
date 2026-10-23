//! `/todo reminders`: the todo list and Apple Reminders kept in step both
//! ways, every list as "All" on the phone shows them, a todo made here going
//! to the list new reminders go to (the user, 2026-10-23: "sync with reminder
//! should happen to 'all' instead of crew section"; v0.28.4–97 kept a `crew`
//! list of its own, folded away by `ekfold`). A todo typed here is on your
//! phone and one ticked there is done here.
//!
//! A thread of its own does the work (`worker`): a pass at once, then one
//! when the todo list settles after a change, when Reminders says it moved,
//! and every minute. A pass reads both sides, decides with no I/O in sight
//! (`plan`), writes Reminders first and crew after (`apply`), and remembers
//! what both agreed (`pairs`). The title, done and the due go across;
//! `@project` and `#assignee` stay crew's. macOS only: EventKit (`ek`) is the
//! way in, and macOS asks once whether crew may.
// The sync's thread is macOS's alone (`worker`), and off it these wait unused.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) mod apply;
mod cmd;
#[cfg(target_os = "macos")]
mod ek;
#[cfg(target_os = "macos")]
mod ekask;
#[cfg(target_os = "macos")]
mod ekfields;
#[cfg(target_os = "macos")]
mod ekfold;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) mod fields;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) mod pairs;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) mod plan;
#[cfg(target_os = "macos")]
mod worker;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc, Mutex};

use crate::applog::LogLevel;

/// Where the sync says what happened: the app's LOG and status line.
pub(crate) type Log = Sender<(LogLevel, String)>;

pub(crate) const MAC_ONLY: &str = "Reminders sync needs macOS";

/// The running sync's stop flag, while one runs.
static RUN: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

/// Start the sync, unless it runs already. Never under test: the first
/// pass asks macOS for Reminders, and a test must not.
pub(crate) fn start(log: Log) {
    #[cfg(target_os = "macos")]
    if !cfg!(test) {
        let mut run = RUN.lock().unwrap_or_else(|e| e.into_inner());
        if run.is_some() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        *run = Some(stop.clone());
        let spawned = std::thread::Builder::new()
            .name("reminders".into())
            .spawn(move || {
                worker::run(&log, &stop);
                ended(&stop);
            });
        if spawned.is_err() {
            *run = None;
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = log.send((LogLevel::Error, MAC_ONLY.into()));
}

/// Stop it after the pass it is in, if it runs.
pub(crate) fn stop() {
    if let Some(s) = RUN.lock().unwrap_or_else(|e| e.into_inner()).take() {
        s.store(true, Ordering::Relaxed);
    }
}

/// The thread ended by itself (access refused): its slot is free, unless a
/// stop and a start have given it to another already.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn ended(stop: &Arc<AtomicBool>) {
    let mut run = RUN.lock().unwrap_or_else(|e| e.into_inner());
    if run.as_ref().is_some_and(|r| Arc::ptr_eq(r, stop)) {
        *run = None;
    }
}
