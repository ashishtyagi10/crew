//! The running task's `/stop`, where a blocking model call can see it.
//!
//! A relay hop's model call is a `block_on` on the worker thread, and the
//! adapter that makes it has no way to be told the task was stopped — so Esc
//! waited for the call to finish (up to its three-minute deadline) before
//! the task noticed. The worker now enters its task's cancel flag here for the
//! life of the task, and a call races its work against it: a stop ends the
//! wait within a tenth of a second.
use std::cell::RefCell;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// What a call that was stopped says.
pub(crate) const STOPPED: &str = "stopped by you";

thread_local! {
    static FLAG: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
}

/// Restores the scope that was there before, when the task's scope ends.
pub(crate) struct Scope(Option<Arc<AtomicBool>>);

impl Drop for Scope {
    fn drop(&mut self) {
        let before = self.0.take();
        FLAG.with(|f| *f.borrow_mut() = before);
    }
}

/// Make `flag` this thread's task cancel flag until the scope drops.
pub(crate) fn enter(flag: Arc<AtomicBool>) -> Scope {
    Scope(FLAG.with(|f| f.borrow_mut().replace(flag)))
}

/// This thread's task cancel flag, if a task is running on it.
pub(crate) fn current() -> Option<Arc<AtomicBool>> {
    FLAG.with(|f| f.borrow().clone())
}

/// `work`, unless `stop` is set first — `None` then. No flag: just `work`.
pub(crate) async fn unless<F: Future>(stop: Option<Arc<AtomicBool>>, work: F) -> Option<F::Output> {
    let Some(stop) = stop else {
        return Some(work.await);
    };
    let stopped = async move {
        while !stop.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    tokio::select! {
        out = work => Some(out),
        () = stopped => None,
    }
}

#[cfg(test)]
#[path = "cancelscope_tests.rs"]
mod tests;
