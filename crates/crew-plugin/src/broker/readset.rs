//! A whole-file write waits until the task has read the file.
//!
//! `sys:write_file` replaces ALL of a file. An agent that never looked at one
//! (it guessed what was in it, or meant to "update" it) wrote back what it
//! imagined, and everything it did not reproduce was gone, under a result
//! that said the write succeeded. Claude Code's `Write` refuses to overwrite a
//! file that was not `Read` first; this is the same rule, kept per task,
//! because what a file held when an earlier task read it says nothing about
//! it now.
//!
//! What counts: a `sys:read_file` of the file, any page or line of it, since
//! the agent that read one page was shown where the rest is; and a
//! `sys:write_file` of it this task, since then the agent wrote every byte.
//! `sys:edit` is not held to it (it matches exact text, so it cannot clobber
//! blind), and does not count toward it, because an edit shows the lines it
//! changed and not the file. A file that does not exist yet is written freely.
//!
//! What it costs: one read the first time a task means to replace a file it
//! never opened, which is exactly the read that was missing.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

#[cfg(test)]
#[path = "readset_tests.rs"]
mod tests;

/// The files one task has read or written whole.
#[derive(Default)]
pub(crate) struct ReadSet {
    seen: Mutex<HashSet<PathBuf>>,
}

impl ReadSet {
    /// A task's set, with nothing in it.
    pub(crate) fn new() -> Arc<Self> {
        Arc::default()
    }

    /// Before a `sys` call runs: a whole-file write over a file this task has
    /// not read is refused, and the file is left as it was. Anything the check
    /// cannot read (no path, bad JSON, nothing there) passes, so the tool's
    /// own error, or its own success, is what the agent sees.
    pub(crate) fn check(&self, tool: &str, args: &str) -> Result<(), String> {
        if tool != "write_file" {
            return Ok(());
        }
        let Some(path) = super::lspwarm::path_arg(args) else {
            return Ok(());
        };
        match key(&path) {
            Some(k) if !self.lock().contains(&k) => Err(format!(
                "{} exists and has not been read this task \u{2014} read it first (sys:read_file), or change part of it with sys:edit",
                path.display()
            )),
            _ => Ok(()),
        }
    }

    /// After a `sys` call succeeded: remember the file a read or a whole-file
    /// write was about.
    pub(crate) fn saw(&self, tool: &str, args: &str) {
        if !matches!(tool, "read_file" | "write_file") {
            return;
        }
        if let Some(k) = super::lspwarm::path_arg(args).as_deref().and_then(key) {
            self.lock().insert(k);
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashSet<PathBuf>> {
        self.seen.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The file `path` names, one key however it was spelled (`./a.rs`, a link to
/// it, its absolute path), or `None` when no file is there. Only a file that
/// exists needs a key: a read that succeeded and a write that just landed
/// both have one, and the check only asks about a file that is there. A
/// relative path resolves against the broker's working directory, as the
/// tools resolve it.
fn key(path: &Path) -> Option<PathBuf> {
    std::fs::metadata(path).ok().filter(|m| m.is_file())?;
    path.canonicalize().ok()
}
