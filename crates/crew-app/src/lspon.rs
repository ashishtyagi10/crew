//! Whether the file viewer asks a language server about the code it opens
//! (see `viewpane::lspjob`). A lock-free flag like `invisibles`: read when a
//! code file lands, set from config load and the settings form.
//!
//! On by default. The viewer's start path also refuses under `cargo test`
//! (see `lspjob`), since a config apply in a parallel test sets this.
use std::sync::atomic::{AtomicBool, Ordering};

static ON: AtomicBool = AtomicBool::new(true);

pub(crate) fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

pub(crate) fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
}
