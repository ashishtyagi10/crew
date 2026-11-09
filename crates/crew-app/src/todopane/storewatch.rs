//! Whether `todos.toml` changed under the store: another process wrote it
//! (`crew todo` run by an agent in a pane, a second crew). The file's
//! modified time and length are noted whenever the store reads or writes it;
//! a stamp that differs from the note is someone else's write. Never under
//! test, where the file is never touched.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use super::{path, read_at, TodoItem};

/// The stamp of the file as the store last read or wrote it.
static SEEN: Mutex<Option<(SystemTime, u64)>> = Mutex::new(None);
/// When the idle check last looked, epoch ms.
static LOOKED: AtomicU64 = AtomicU64::new(0);
/// How often the idle check stats the file.
const EVERY_MS: u64 = 1000;

fn stamp() -> Option<(SystemTime, u64)> {
    let m = std::fs::metadata(path()?).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// Note the file as it is now: the store just read or wrote it.
pub(super) fn seen() {
    if !cfg!(test) {
        *SEEN.lock().unwrap_or_else(|e| e.into_inner()) = stamp();
    }
}

/// Whether a second has passed since the idle check last looked.
pub(super) fn due() -> bool {
    if cfg!(test) {
        return false;
    }
    let now = crate::chattime::unix_now_ms();
    let last = LOOKED.load(Ordering::Relaxed);
    now.saturating_sub(last) >= EVERY_MS
        && LOOKED
            .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
}

/// Read the file into `items` when someone else changed it; whether it did.
/// A file that will not read is noted and left alone: the list in memory is
/// better than none, and the next write puts a good file back.
pub(super) fn reload(items: &mut Vec<TodoItem>) -> bool {
    if cfg!(test) {
        return false;
    }
    // A file that is gone is not an empty list: the next write restores it.
    let Some(now) = stamp().map(Some) else {
        return false;
    };
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if now == *seen {
        return false;
    }
    *seen = now;
    match read_at(path().as_deref()) {
        Ok(fresh) if fresh != *items => {
            *items = fresh;
            true
        }
        _ => false,
    }
}
