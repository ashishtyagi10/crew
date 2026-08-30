//! The viewer pane's model: a path, a load state, and where it is scrolled.
//! Deliberately thin — rendering lives in `render`, key decoding in `keys`.
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use super::blamejob::Blame;
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
    /// Rows `]` / `[` step between, in this rendering (see
    /// [`super::outline`]). Empty for a rung with no structure to step.
    pub marks: Vec<super::outline::Mark>,
    /// Pictures this rendering reserved room for (`![alt](src)`), in rendered
    /// rows — filled by the markdown rung, empty everywhere else. Part of the
    /// cache because the rows are a property of THIS wrap at THIS width.
    pub pictures: Vec<crate::chatmd::Picture>,
    /// Columns the blame gutter claimed in this rendering. Part of the cache
    /// key in spirit: turning blame on or off changes the width the text was
    /// wrapped at, so the cache must be rebuilt, not merely re-decorated.
    pub blame_w: usize,
    /// Whether this rendering is the side-by-side review. Part of the cache
    /// key for the same reason `blame_w` is: the two rungs wrap at different
    /// widths, so one cannot be turned into the other.
    pub split: bool,
    /// Whether the invisibles were revealed in this rendering. Part of the
    /// cache key: the toggle changes the TEXT (a revealed tab wears an arrow
    /// in its first column), so the rendering has to be rebuilt, not
    /// recoloured.
    pub invisibles: bool,
    /// The theme the ink in `lines` was taken from. Part of the cache key
    /// because these lines carry BAKED colours — `t.ink`, `text_muted`, the
    /// whole `chatink` syntax ladder — decided once when the rendering was
    /// built. Without it a `/theme` (or the auto theme flipping at dusk, or
    /// the OS switching appearance) left every open viewer wearing the old
    /// palette's ink until something else happened to resize the pane: on a
    /// dark-to-light switch that is a file drawn in near-white on paper.
    pub theme: crew_theme::ThemeId,
}

pub(crate) struct ViewPane {
    pub path: PathBuf,
    pub state: LoadState,
    /// Rows scrolled from the top, clamped to content by `clamp_scroll`.
    pub scroll: usize,
    /// `s`: show the text unrendered. The escape hatch for when the render is
    /// the thing being debugged.
    pub raw: bool,
    /// The cursor in the render, when this document is being EDITED rather
    /// than read (see [`super::caret`]). `None` in a viewer pane: a pane is a
    /// window onto a file, and the arrow keys there scroll it. A document
    /// window turns it on, which is what makes the window an editor.
    pub caret: Option<super::caret::Caret>,
    /// The BYTE the caret is on — its durable identity. The rendered position
    /// above is derived from this and the current layout, and a re-wrap
    /// (a resize, an edit) throws that position away and finds this offset
    /// again. Keeping the row would put the cursor on a different word every
    /// time the window changed width.
    pub caret_at: Option<u32>,
    /// Edited since it was read or last saved.
    pub dirty: bool,
    /// `v`: lay a diff out side by side rather than unified (see
    /// [`super::diffsplit`]). Per pane rather than a setting: it is a way of
    /// reading THIS review at THIS width, and a pane too narrow to hold two
    /// columns falls back on its own.
    pub split: bool,
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
    /// Who last touched each line ([`super::blame`]), when `/blame` has been
    /// asked and the answer has arrived. `Off` — the default — is a viewer
    /// with no blame column at all.
    pub blame: Blame,
    /// A 1-based line to land on once the load arrives — from a clicked
    /// `path:line` reference. The read is on a worker thread, so the pane
    /// exists before there is anything to scroll; this is applied when the
    /// text lands and then cleared.
    pub goto: Option<usize>,
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
            caret: None,
            caret_at: None,
            dirty: false,
            split: false,
            search: None,
            cache: RefCell::new(None),
            editor_born: None,
            ephemeral: false,
            blame: Blame::default(),
            goto: None,
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
        // A `path:line` click asked for a line, and this is the first moment
        // there is one to scroll to. Landing it at the TOP of the window
        // rather than centring it: the lines after the one you were sent to
        // are the ones you came to read.
        if let Some(n) = self.goto.take() {
            self.scroll = n.saturating_sub(1);
        }
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
        // …and so is a blame: the file is being re-read because it changed,
        // and a per-line answer about the old text would label the new one.
        self.blame = Blame::Off;
    }
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
