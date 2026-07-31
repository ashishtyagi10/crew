//! The viewer pane's model: a path, a load state, and where it is scrolled.
//! Deliberately thin — rendering lives in `render`, key decoding in `keys`.
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use super::detect::Format;
use super::load::{self, Loaded};
use super::search::Search;

/// Where a pane is between "you pressed the key" and "the bytes are here".
/// `Loading` holds the channel so `poll` can drain it without the app owning
/// a side table of in-flight loads.
pub(crate) enum LoadState {
    Loading { rx: Receiver<load::LoadDone> },
    Ready { format: Format, loaded: Loaded },
    Failed(String),
}

/// Wrapped lines for the last width this pane rendered at. Rebuilt on a width
/// change, a reload, or the `s` toggle — never per frame.
pub(crate) struct ViewCache {
    pub cols: u16,
    pub raw: bool,
    pub lines: Vec<crate::chatbody::CardLine>,
}

pub(crate) struct ViewPane {
    pub path: PathBuf,
    pub state: LoadState,
    /// Rows scrolled from the top, clamped to content by `clamp_scroll`.
    pub scroll: usize,
    /// `s`: show the text unrendered. The escape hatch for when the render is
    /// the thing being debugged.
    pub raw: bool,
    /// A live `/` search: `None` when no search is in progress. Cleared on
    /// `reload` — a search over text that is about to change is stale.
    pub search: Option<Search>,
    pub(crate) cache: RefCell<Option<ViewCache>>,
    /// The `born_ms` of the terminal pane `e` spawned to edit this file, if
    /// any is outstanding. `poll_panes` clears it and calls `reload` once
    /// that pane's `cmd` goes back to `None` (the editor exited) — see
    /// `poll::reload_views_after_edit`. Identified by `born_ms` rather than
    /// pane index: indices shift the moment any pane closes.
    pub editor_born: Option<u64>,
    /// Set by `/about` and `??` (`openview::spawn_about_pane`,
    /// `askbar::absorb_explain_result`) for the viewer they open on a
    /// SYNTHETIC temp file — a changelog or an explanation, not something
    /// the user asked to view. Fix 4: `session_panes`/`had_restorable` skip
    /// these, so quitting with only one of these open can't overwrite a
    /// saved multi-pane session with a changelog viewer.
    pub ephemeral: bool,
}

impl ViewPane {
    /// Open `path`: the worker starts immediately and the pane is on screen
    /// before a single byte has been read.
    pub(crate) fn open(path: PathBuf) -> Self {
        let rx = load::start(path.clone());
        Self {
            path,
            state: LoadState::Loading { rx },
            scroll: 0,
            raw: false,
            search: None,
            cache: RefCell::new(None),
            editor_born: None,
            ephemeral: false,
        }
    }

    /// Test-only: `pane_tests.rs` reads this directly rather than matching
    /// `state` itself, in every one of its load/reload/failure assertions.
    /// Nothing in production code needs this predicate — `poll.rs`'s drain
    /// and `render.rs`'s `for_state` both match on `state` directly instead
    /// — so `#[cfg(test)]` makes that true rather than the dead-code lint
    /// firing on genuinely-used test-support code.
    #[cfg(test)]
    pub(crate) fn loading(&self) -> bool {
        matches!(self.state, LoadState::Loading { .. })
    }

    /// Drain the worker channel. Returns `true` on the tick the state changed,
    /// which is what tells `poll_panes` to redraw.
    pub(crate) fn poll(&mut self) -> bool {
        let LoadState::Loading { rx, .. } = &self.state else {
            return false;
        };
        let done = match rx.try_recv() {
            Ok(done) => done,
            // The worker died without sending — a panic in the load thread.
            // Nothing will ever arrive, so settle as Failed rather than
            // staying Loading forever: a pane stuck showing "loading…" with
            // no way to fail is indistinguishable from one still waiting on
            // a slow disk — the user would never learn it isn't coming.
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.state = LoadState::Failed(format!(
                    "{}: loader stopped unexpectedly",
                    self.path.display()
                ));
                self.cache.replace(None);
                return true;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => return false,
        };
        self.state = match done.result {
            Ok(loaded) => LoadState::Ready {
                format: done.format,
                loaded,
            },
            Err(msg) => LoadState::Failed(msg),
        };
        self.cache.replace(None);
        true
    }

    /// Re-read from disk, keeping the pane in place. Used by `r` and by the
    /// `$EDITOR` handoff when the editor exits.
    pub(crate) fn reload(&mut self) {
        let rx = load::start(self.path.clone());
        self.state = LoadState::Loading { rx };
        self.cache.replace(None);
        // A search over text that's about to change is stale the instant the
        // reload lands — drop it rather than leave hits pointing at lines
        // that no longer say what they used to.
        self.search = None;
    }
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
