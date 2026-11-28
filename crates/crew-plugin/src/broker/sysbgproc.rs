//! One background process and the one rule about signalling its group.
//!
//! A group is addressed by a bare pgid number. Once its leader has been waited
//! on and the group is empty, the OS may hand that number to an unrelated
//! process, so a signal sent later could kill a stranger. Hence each [`Proc`]
//! is swept exactly once, and nothing signals it after that:
//!
//! - the waiter thread sweeps the moment the leader exits (orphans it left are
//!   still TERM/KILLed, and the number cannot have been reused yet because
//!   nobody has signalled, let alone freed, anything since);
//! - `sys:kill`, the task ending and the broker exiting skip a swept proc.
//!
//! A sliver remains between `wait()` returning and the flag being set; it is
//! microseconds, and the group is normally still occupied by its orphans.
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::sysbgbuf::Shared;
use super::sysrun::signal_group;

pub(super) struct Proc {
    pub(super) id: u32,
    pub(super) pgid: u32,
    pub(super) cmd: String,
    pub(super) out: Arc<Shared>,
    /// Set once the group was swept: from then on its pgid is never signalled.
    pub(super) swept: AtomicBool,
    /// Signals actually sent to the group (tests assert none after a sweep).
    pub(super) signals: AtomicU32,
}

/// Every proc started and not yet swept, process-wide, for [`reap_all`].
static LIVE: Mutex<Vec<Arc<Proc>>> = Mutex::new(Vec::new());

pub(super) fn register(p: &Arc<Proc>) {
    LIVE.lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Arc::clone(p));
}

impl Proc {
    /// One signal to the group, unless it is already known gone.
    pub(super) fn signal(&self, sig: &str) {
        if !self.swept.load(Ordering::SeqCst) {
            self.signals.fetch_add(1, Ordering::SeqCst);
            signal_group(sig, self.pgid);
        }
    }

    pub(super) fn sweep(self: &Arc<Self>) {
        sweep(std::slice::from_ref(self));
    }
}

/// TERM every not-yet-swept group, a beat for the polite, then KILL, and mark
/// them swept. Claiming the flag first makes it idempotent and race-free.
pub(super) fn sweep(procs: &[Arc<Proc>]) {
    let mine: Vec<&Arc<Proc>> = procs
        .iter()
        .filter(|p| !p.swept.swap(true, Ordering::SeqCst))
        .collect();
    let send = |sig: &str| {
        mine.iter().for_each(|p| {
            p.signals.fetch_add(1, Ordering::SeqCst);
            signal_group(sig, p.pgid);
        })
    };
    send("-TERM");
    std::thread::sleep(Duration::from_millis(20));
    send("-KILL");
    LIVE.lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|l| !mine.iter().any(|p| Arc::ptr_eq(l, p)));
}

/// The broker is going away: nothing started here may outlive it.
pub(crate) fn reap_all() {
    let all: Vec<Arc<Proc>> = LIVE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    sweep(&all);
}
