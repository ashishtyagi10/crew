//! The one global todo list, persisted as `todos.toml` beside the config.
//!
//! A process-wide singleton (the `usageledger` pattern): every pane and the
//! due-toast ticker read and write through it, so two open todo panes can
//! never diverge in one process. Disk writes are small synchronous
//! rewrite-on-save TOML (the `session.toml` precedent); concurrent crew
//! *processes* are last-write-wins, documented in the goal.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::item::TodoItem;

#[derive(Serialize, Deserialize, Default)]
struct TodoFile {
    #[serde(default)]
    items: Vec<TodoItem>,
}

/// `todos.toml` beside the config file.
pub(crate) fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("todos.toml"))
}

/// Load items from `p`; a missing or unreadable file is an empty list.
pub(crate) fn load_at(p: Option<&Path>) -> Vec<TodoItem> {
    let Some(p) = p else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(p) else {
        return Vec::new();
    };
    toml::from_str::<TodoFile>(&text).unwrap_or_default().items
}

/// Rewrite `p` with `items` (creating the config dir if needed).
pub(crate) fn save_at(p: Option<&Path>, items: &[TodoItem]) {
    let Some(p) = p else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = toml::to_string(&TodoFile {
        items: items.to_vec(),
    }) {
        let _ = std::fs::write(p, s);
    }
}

static STORE: Mutex<Option<Vec<TodoItem>>> = Mutex::new(None);
/// Bumped on every mutation; panes resync their snapshot when it moves.
static REV: AtomicU64 = AtomicU64::new(1);

pub(crate) fn revision() -> u64 {
    REV.load(Ordering::Relaxed)
}

/// A clone of the current list (loading the file on first touch).
pub(crate) fn snapshot() -> Vec<TodoItem> {
    let mut g = STORE.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(|| load_at(path().as_deref())).clone()
}

/// Apply `f` to the list, persist, and bump the revision. Never writes the
/// real file from the test harness (the `CrewConfig::save` rule).
pub(crate) fn mutate<R>(f: impl FnOnce(&mut Vec<TodoItem>) -> R) -> R {
    let mut g = STORE.lock().unwrap_or_else(|e| e.into_inner());
    let items = g.get_or_insert_with(|| load_at(path().as_deref()));
    let r = f(items);
    if !cfg!(test) {
        save_at(path().as_deref(), items);
    }
    REV.fetch_add(1, Ordering::Relaxed);
    r
}

/// Items whose due instant has passed and haven't been toasted yet: mark
/// them notified (persisted — a restart must not re-toast the backlog) and
/// return their titles. The common every-minute case (nothing due) takes
/// only the read lock and never touches disk.
pub(crate) fn take_due(now_ms: u64) -> Vec<String> {
    let any_due = {
        let mut g = STORE.lock().unwrap_or_else(|e| e.into_inner());
        g.get_or_insert_with(|| load_at(path().as_deref()))
            .iter()
            .any(|it| due_now(it, now_ms))
    };
    if !any_due {
        return Vec::new();
    }
    mutate(|items| {
        items
            .iter_mut()
            .filter(|it| due_now(it, now_ms))
            .map(|it| {
                it.notified = true;
                it.title.clone()
            })
            .collect()
    })
}

fn due_now(it: &TodoItem, now_ms: u64) -> bool {
    !it.done && !it.notified && it.due_ms.is_some_and(|d| d <= now_ms)
}

/// Serialise tests that drive the process-global store: holds a test lock
/// while replacing the in-memory list (disk is never touched under test —
/// see [`mutate`]). Keep the guard alive for the whole test.
#[cfg(test)]
pub(crate) fn test_guard(items: Vec<TodoItem>) -> std::sync::MutexGuard<'static, ()> {
    static TEST: Mutex<()> = Mutex::new(());
    let g = TEST.lock().unwrap_or_else(|e| e.into_inner());
    *STORE.lock().unwrap_or_else(|e| e.into_inner()) = Some(items);
    REV.fetch_add(1, Ordering::Relaxed);
    g
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
