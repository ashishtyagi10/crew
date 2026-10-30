//! An agent CLI the user stopped is ended where it stands.
//!
//! A relay dial blocks its thread until the CLI exits, so the harness's race
//! against `/stop` (`cancelscope`) never reached it: Esc ended the task on
//! screen while `claude -p` went on reading and editing the tree for as long
//! as its run took — up to the ten-minute idle window. The read loops now
//! look at the task's flag on every 50 ms tick and end the CLI once it is
//! set.
//!
//! TERM first: Claude Code ends its own Bash tool children on TERM and exits
//! (measured 2026-10-30, a `sleep 47` it was running was gone with it). The
//! broker's process group is the backstop for a CLI that does not: a pane
//! closing kills that whole group, so the CLI stays in it rather than in a
//! group of its own a stop could take — one that would outlive the pane.
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::cancelscope;

/// How long a CLI is given to end its children after TERM before KILL.
const GRACE: Duration = Duration::from_secs(2);

/// The running task's stop, as a read loop checks it.
pub(super) struct Stop(Option<Arc<AtomicBool>>);

impl Stop {
    /// This thread's task stop; outside a task there is none to press.
    pub(super) fn current() -> Self {
        Self(cancelscope::current())
    }

    pub(super) fn pressed(&self) -> bool {
        self.0.as_ref().is_some_and(|f| f.load(Ordering::Relaxed))
    }
}

/// End `child` — TERM, then KILL after [`GRACE`] — and say why the dial
/// came back without a reply.
pub(super) fn end(child: &mut Child) -> String {
    #[cfg(unix)]
    // SAFETY: signalling a pid this process spawned and has not yet reaped.
    unsafe {
        libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
    }
    let t0 = Instant::now();
    while t0.elapsed() < GRACE {
        if let Ok(Some(_)) = child.try_wait() {
            return cancelscope::STOPPED.to_string();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();
    cancelscope::STOPPED.to_string()
}

#[cfg(all(test, unix))]
#[path = "clistop_tests.rs"]
mod tests;
