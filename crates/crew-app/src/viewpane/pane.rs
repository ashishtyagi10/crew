//! The viewer pane's model: a path, a load state, and where it is scrolled.
//! Deliberately thin — rendering lives in `render`, key decoding in `keys`.
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use super::detect::Format;
use super::load::{self, Loaded};

/// Where a pane is between "you pressed the key" and "the bytes are here".
/// `Loading` holds the channel so `poll` can drain it without the app owning
/// a side table of in-flight loads.
pub(crate) enum LoadState {
    Loading {
        since_ms: u64,
        rx: Receiver<load::LoadDone>,
    },
    Ready {
        format: Format,
        loaded: Loaded,
    },
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
    pub(crate) cache: RefCell<Option<ViewCache>>,
}

impl ViewPane {
    /// Open `path`: the worker starts immediately and the pane is on screen
    /// before a single byte has been read.
    pub(crate) fn open(path: PathBuf) -> Self {
        let rx = load::start(path.clone());
        Self {
            path,
            state: LoadState::Loading {
                since_ms: crate::anim::now_ms(),
                rx,
            },
            scroll: 0,
            raw: false,
            cache: RefCell::new(None),
        }
    }

    pub(crate) fn loading(&self) -> bool {
        matches!(self.state, LoadState::Loading { .. })
    }

    /// Drain the worker channel. Returns `true` on the tick the state changed,
    /// which is what tells `poll_panes` to redraw.
    pub(crate) fn poll(&mut self) -> bool {
        let LoadState::Loading { rx, .. } = &self.state else {
            return false;
        };
        let Ok(done) = rx.try_recv() else {
            return false;
        };
        self.state = match done.result {
            Ok(loaded) => LoadState::Ready {
                format: done.format,
                loaded,
            },
            Err(msg) => LoadState::Failed(msg),
        };
        self.cache.replace(None);
        self.scroll = 0;
        true
    }

    /// Re-read from disk, keeping the pane in place. Used by `r` and by the
    /// `$EDITOR` handoff when the editor exits.
    pub(crate) fn reload(&mut self) {
        let rx = load::start(self.path.clone());
        self.state = LoadState::Loading {
            since_ms: crate::anim::now_ms(),
            rx,
        };
        self.cache.replace(None);
    }
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
