//! The servers this process has alive right now, so `/lsp` can say so. A
//! client registers when it has spawned and leaves when it is dropped; the
//! list is per process, which is exactly the question the status card asks.
use std::path::PathBuf;
use std::sync::Mutex;

static RUNNING: Mutex<Vec<(String, PathBuf)>> = Mutex::new(Vec::new());

fn lock() -> std::sync::MutexGuard<'static, Vec<(String, PathBuf)>> {
    RUNNING.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn register(lang: &str, root: &std::path::Path) {
    lock().push((lang.to_string(), root.to_path_buf()));
}

pub(crate) fn unregister(lang: &str, root: &std::path::Path) {
    let mut list = lock();
    if let Some(i) = list.iter().position(|(l, r)| l == lang && r == root) {
        list.remove(i);
    }
}

/// `(language, root)` for every live server, in start order.
pub fn list() -> Vec<(String, PathBuf)> {
    lock().clone()
}
