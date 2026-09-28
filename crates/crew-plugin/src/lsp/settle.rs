//! Bringing a server's copy of a file level with the disk, and waiting for
//! its word on it — shared by the `lsp:diagnostics` tool, which waits as long
//! as a server needs, and the note an edit's own result carries
//! (`broker/editdiag.rs`), which may wait no longer than the edit can.
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crew_lsp::docsync::Synced;
use crew_lsp::{servers, Diagnostic, Severity};

use super::{tools, LspHost, SETTLE, TIMEOUT};

/// How long `diagnostics` waits for a publish, by what the call had to tell
/// the server. Text it has not spoken about yet waits for its word, which may
/// be a first index. Unchanged text already has it when a publish was queued,
/// and otherwise gives a `cargo check` still running from the last save a
/// moment. A change waits only SETTLE: rust-analyzer re-publishes a change
/// that moves the diagnostics within 11–180 ms (measured), and publishes
/// NOTHING for one that leaves them as they were — so TIMEOUT made a clean
/// edit to a clean file wait 15 s for the answer it already had.
pub(super) fn diag_wait(synced: Synced, queued: bool) -> Option<Duration> {
    match synced {
        Synced::Opened => Some(TIMEOUT),
        Synced::Changed => Some(SETTLE),
        Synced::Unchanged if queued => None,
        Synced::Unchanged => Some(SETTLE),
    }
}

impl LspHost {
    /// Send the running server for `key` the disk's copy of `path` if its
    /// own has moved on. The disk is read on every call, not just the first:
    /// an agent edits between questions, and a server answers about the copy
    /// it was last sent. A copy that moved on is resent whole, with a save;
    /// a first look for diagnostics (`save_opened`) saves too, so
    /// rust-analyzer's `cargo check` (the type errors) runs on it — a hover
    /// does not need one. A server that failed the sync is dropped, so the
    /// next call starts fresh.
    pub(super) fn resync(
        &mut self,
        key: &(String, PathBuf),
        uri: &str,
        path: &Path,
        save_opened: bool,
    ) -> Result<Synced, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(c) = self.clients.get_mut(key) else {
            return Err(format!("no {} server running", key.0));
        };
        let synced = c.sync(uri, &key.0, &text).and_then(|s| {
            if s == Synced::Opened && save_opened {
                c.did_save(uri, &text)?;
            }
            Ok(s)
        });
        synced.inspect_err(|_| {
            self.clients.remove(key);
        })
    }

    /// What the server already running for `path` has said about it since
    /// the disk moved on that it had not said before, heard within `cap`, so
    /// whoever holds the host's lock lets go of it by then.
    ///
    /// New, because rust-analyzer's publishes mix two clocks: its own
    /// checks, redone within milliseconds of a change, and `cargo check`'s,
    /// redone only when the check it starts on the save finishes (≈ 0.3 s on
    /// a small crate, seconds on a large one). Until then every publish
    /// carries the old text's `rustc` errors — measured, the first word after
    /// a fix still held the error just fixed, and a note calling that current
    /// would send the agent back to fix it again. Nor is the first word the
    /// answer: on an open it is the syntax pass, empty, 5 ms before the type
    /// error. So the ask stops at the first publish with an error that was
    /// not there before, or at SETTLE; an error that was there before is the
    /// old text's word, or was there before this write, and `lsp:diagnostics`
    /// and the task's end still list it.
    ///
    /// `None` when no server is running for it yet: starting one holds the
    /// lock through its spawn and handshake, well past any cap, and one just
    /// started is still indexing (the task's first write is starting it,
    /// `broker/lspwarm.rs`). `None` too when the copy could not be brought
    /// level.
    pub fn new_diagnostics(&mut self, path: &Path, cap: Duration) -> Option<Vec<Diagnostic>> {
        let path = tools::absolute(&path.to_string_lossy());
        let lang = servers::lang_of(&path)?;
        let key = (lang.to_string(), crew_lsp::root::for_file(&path));
        if !self.clients.contains_key(&key) {
            return None;
        }
        let uri = crew_lsp::uri::from_path(&path);
        self.drain(&key, &uri);
        let before = self.diags.get(&uri).cloned().unwrap_or_default();
        // A file the server had not opened waits no longer than a change:
        // a server that is ready speaks within milliseconds of either, and
        // one still indexing would not speak within an edit's bound anyway.
        let wait = match self.resync(&key, &uri, &path, true).ok()? {
            Synced::Opened | Synced::Changed => EDIT_SETTLE.min(cap),
            // The server already had this text: it has nothing new to say.
            Synced::Unchanged => Duration::ZERO,
        };
        let deadline = Instant::now() + wait;
        let is_new = |d: &Diagnostic| !before.contains(d);
        while self.wait_publish(
            &key,
            &uri,
            deadline.saturating_duration_since(Instant::now()),
        ) {
            let heard = self.diags.get(&uri).map_or(&[][..], Vec::as_slice);
            if heard
                .iter()
                .any(|d| d.severity == Severity::Error && is_new(d))
            {
                break;
            }
        }
        let heard = self.diags.get(&uri).map_or(&[][..], Vec::as_slice);
        Some(heard.iter().filter(|d| is_new(d)).cloned().collect())
    }
}

/// How long an edit waits for its file's new errors. Measured with
/// rust-analyzer: a new error was published 12 ms (type) and 234 ms
/// (borrow, from `cargo check` on a small crate) after the write, and a clean
/// edit is never published at all — so every clean edit pays this in full.
/// SETTLE's 1.5 s made eight clean edits cost twelve seconds.
const EDIT_SETTLE: Duration = Duration::from_millis(700);

#[cfg(test)]
#[path = "settle_tests.rs"]
mod tests;
