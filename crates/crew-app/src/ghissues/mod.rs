//! A todo tagged `@project` becomes a GitHub issue in that project's repo,
//! and the two stay in step both ways: tick the todo and the issue closes,
//! close the issue and the todo ticks, rename either and the other follows.
//! The user, 2026-11-09: "if a todo item is associated with a project, crew
//! should create the github issue related with that in relevant github
//! repository".
//!
//! On by default for every `@project` that resolves to a github.com
//! checkout; `/todo issues off` stops it, kept in `ghissues.toml` beside
//! `todos.toml`. A thread of its own does the work ([`worker`]) through the
//! `gh` CLI ([`gh`]), so crew holds no token. Not under test.
mod cmd;
mod gh;
pub(crate) mod plan;
mod repo;
mod worker;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::applog::LogLevel;

pub(crate) type Log = Sender<(LogLevel, String)>;

#[derive(Serialize, Deserialize, Default)]
struct Switch {
    #[serde(default)]
    off: bool,
}

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("ghissues.toml"))
}

/// Whether the user turned the sync off.
pub(crate) fn is_off() -> bool {
    let text = path().and_then(|p| std::fs::read_to_string(p).ok());
    text.and_then(|t| toml::from_str::<Switch>(&t).ok())
        .is_some_and(|s| s.off)
}

fn set_off(off: bool) {
    let Some(p) = path().filter(|_| !cfg!(test)) else {
        return;
    };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = toml::to_string(&Switch { off }) {
        let _ = std::fs::write(p, s);
    }
}

static RUN: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

/// Start the sync at launch, unless it is off or runs already.
pub(crate) fn start(log: Log) {
    if cfg!(test) || is_off() {
        return;
    }
    let mut run = RUN.lock().unwrap_or_else(|e| e.into_inner());
    if run.is_some() {
        return;
    }
    let stop = Arc::new(AtomicBool::new(false));
    *run = Some(stop.clone());
    let spawned = std::thread::Builder::new()
        .name("ghissues".into())
        .spawn(move || worker::run(&log, &stop));
    if spawned.is_err() {
        *run = None;
    }
}

/// Stop it after the pass it is in.
pub(crate) fn stop() {
    if let Some(s) = RUN.lock().unwrap_or_else(|e| e.into_inner()).take() {
        s.store(true, Ordering::Relaxed);
    }
}
