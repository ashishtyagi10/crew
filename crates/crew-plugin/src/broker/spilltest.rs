//! The test seam for `spill`: where a test's spills go, and the lock that
//! keeps tests that spill, or turn spilling off, from racing each other.
//!
//! The directory is per THREAD, not a process-wide variable: lib tests run in
//! parallel on one working directory, and a spill root in the environment
//! would send one test's output into another's temp dir, or into the crate.
//! A test that never takes a [`Guard`] spills nothing and sees the result as
//! it always was. The lock is for `CREW_SPILL`, the one piece of spilling that
//! is process-wide: a test that sets it holds a guard, and so does every test
//! that expects a spill, so neither sees the other's setting.
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

thread_local! {
    static ROOT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

static LOCK: Mutex<()> = Mutex::new(());

/// The directory this thread's test spills into, if it took a guard.
pub(super) fn root() -> Option<PathBuf> {
    ROOT.with(|r| r.borrow().clone())
}

/// A fresh, empty directory this thread spills into until the guard drops,
/// held under the spill lock; dropped, the directory goes with it.
pub(crate) struct Guard {
    _lock: MutexGuard<'static, ()>,
    dir: PathBuf,
}

impl Guard {
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        ROOT.with(|r| *r.borrow_mut() = None);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub(crate) fn guard(tag: &str) -> Guard {
    static N: AtomicUsize = AtomicUsize::new(0);
    let lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("crew-spill-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    ROOT.with(|r| *r.borrow_mut() = Some(dir.clone()));
    Guard { _lock: lock, dir }
}
