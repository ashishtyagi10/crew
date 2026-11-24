//! Which of Far's recent folders (`farrecent`) are gone from disk — drawn muted and struck in
//! the nav (`navfarrow`) rather than found out only by a click.
//!
//! A stat per row per frame is twenty syscalls a frame for an answer that changes once in a
//! blue moon, so the answer is kept here beside the paths: a look starts when the nav turns to
//! Far, then at most once every [`EVERY_MS`] while it stays there. The look itself runs on a
//! worker thread — a stat of a hung network mount or a sleeping disk can take seconds, and the
//! winit thread must never wait on one. The tick only starts a look (one at a time) and lands
//! a finished one, so a gone folder is drawn struck a moment after the look begins.
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Mutex;

/// The longest a gone folder goes undrawn as gone while the Far section shows.
pub(crate) const EVERY_MS: u64 = 3_000;

/// When the last look started (`None`: not since the section last showed), the folders gone
/// at the last look that landed, and the look in flight, if one is.
struct Gone {
    started: Option<u64>,
    gone: Vec<PathBuf>,
    look: Option<Receiver<Vec<PathBuf>>>,
}

static GONE: Mutex<Gone> = Mutex::new(Gone {
    started: None,
    gone: Vec::new(),
    look: None,
});

/// Land a finished look, then start another when the section `shows` and the last one started
/// [`EVERY_MS`] ago or before it showed. Never waits on a stat. `true` when the gone set changed.
pub(crate) fn refresh(now_ms: u64, shows: bool) -> bool {
    refresh_with(now_ms, shows, |d| d.is_dir())
}

/// [`refresh`] with the stat a test can stand in for.
pub(crate) fn refresh_with(now_ms: u64, shows: bool, is_dir: fn(&Path) -> bool) -> bool {
    let mut g = GONE.lock().unwrap_or_else(|p| p.into_inner());
    // Land first, so a look that finished is drawn this tick, not after the throttle.
    let changed = land(&mut g);
    if !shows {
        // Hidden: the next showing looks at once.
        g.started = None;
        return changed;
    }
    let due = g
        .started
        .is_none_or(|t| now_ms.saturating_sub(t) >= EVERY_MS);
    if due && g.look.is_none() {
        // The list is read here, on the winit thread; the worker only stats and sends.
        let dirs = crate::farrecent::list();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let gone: Vec<PathBuf> = dirs.into_iter().filter(|d| !is_dir(d)).collect();
            let _ = tx.send(gone);
        });
        g.started = Some(now_ms);
        g.look = Some(rx);
    }
    changed
}

/// Take a finished look's answer, if one is in. `true` when the gone set changed.
fn land(g: &mut Gone) -> bool {
    let Some(rx) = &g.look else {
        return false;
    };
    match rx.try_recv() {
        Ok(gone) => {
            g.look = None;
            let changed = g.gone != gone;
            g.gone = gone;
            changed
        }
        // A worker that died hands nothing back: free the slot so looks go on.
        Err(TryRecvError::Disconnected) => {
            g.look = None;
            false
        }
        Err(TryRecvError::Empty) => false,
    }
}

/// The folders gone at the last look that landed.
pub(crate) fn gone() -> Vec<PathBuf> {
    GONE.lock().unwrap_or_else(|p| p.into_inner()).gone.clone()
}

/// Whether a look is in flight.
#[cfg(test)]
pub(crate) fn looking() -> bool {
    GONE.lock()
        .unwrap_or_else(|p| p.into_inner())
        .look
        .is_some()
}

/// Wait (up to 5 s) for the look in flight to land — for tests that want the answer drawn.
/// Lands only: the throttle's clock is left as the tick set it.
#[cfg(test)]
pub(crate) fn settle() {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while looking() && std::time::Instant::now() < end {
        std::thread::sleep(std::time::Duration::from_millis(2));
        land(&mut GONE.lock().unwrap_or_else(|p| p.into_inner()));
    }
}

/// Whether `dir` was gone at the last look.
#[cfg(test)]
pub(crate) fn is_gone(dir: &Path) -> bool {
    GONE.lock()
        .unwrap_or_else(|p| p.into_inner())
        .gone
        .iter()
        .any(|d| d == dir)
}

/// Nothing gone, nothing looked at, no look in flight — with `farrecent::clear`.
#[cfg(test)]
pub(crate) fn reset() {
    let mut g = GONE.lock().unwrap_or_else(|p| p.into_inner());
    *g = Gone {
        started: None,
        gone: Vec::new(),
        look: None,
    };
}

#[cfg(test)]
#[path = "fargone_tests.rs"]
mod tests;
