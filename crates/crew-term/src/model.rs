use std::sync::atomic::Ordering;

use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;

use crate::color::{default_bg, default_fg, resolve_color};
use crate::listener::TermEvents;

#[derive(Clone, Copy, Debug)]
pub struct GridSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderCell {
    pub col: u16,
    pub row: u16,
    pub c: char,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
    pub bold: bool,
    pub italic: bool,
}

pub trait TermModel {
    fn feed(&mut self, bytes: &[u8]);
    /// Render cells; `focused` brightens the block cursor (dim otherwise).
    fn cells(&self, focused: bool) -> Vec<RenderCell>;
    fn resize(&mut self, size: GridSize);
}

// alacritty_terminal needs a Dimensions impl describing the viewport.
#[derive(Clone, Copy)]
struct Dims {
    cols: usize,
    rows: usize,
}

impl Dimensions for Dims {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

// Shared core: a Term + an ANSI processor. Used by HeadlessTerm and PtyTerm.
pub(crate) struct TermCore {
    term: Term<TermEvents>,
    parser: Processor,
    events: TermEvents,
    /// Sniffs OSC 7 working-directory reports — which the ANSI parser ignores —
    /// so a `cd` inside the pane can retitle it.
    osc7: crate::osc7::Osc7Scanner,
    /// Sniffs DECSET 2031 (color-scheme notifications) — also invisible to
    /// the ANSI parser — so theme flips can be pushed to opted-in TUIs.
    scheme: crate::schemenotify::SchemeNotify,
    /// Where the drag began, and its kind. Kept because a selection's sides
    /// depend on the drag's DIRECTION, which only the anchor can tell us, and
    /// `Selection` doesn't hand its anchor back. See `sel_update`.
    sel_anchor: Option<(Point, SelectionType)>,
}

impl TermCore {
    pub(crate) fn new(size: GridSize) -> Self {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        let events = TermEvents::default();
        let term = Term::new(Config::default(), &dims, events.clone());
        Self {
            term,
            parser: Processor::new(),
            events,
            osc7: crate::osc7::Osc7Scanner::default(),
            scheme: crate::schemenotify::SchemeNotify::default(),
            sel_anchor: None,
        }
    }

    /// The current program-set window title (empty if none).
    pub(crate) fn title(&self) -> String {
        self.events.title.lock().unwrap().clone()
    }

    /// The directory reported by the program (OSC 7) if it changed since the last
    /// call, else `None`.
    pub(crate) fn take_cwd(&mut self) -> Option<std::path::PathBuf> {
        self.osc7.take_cwd()
    }

    /// Take any pending OSC 52 clipboard-store text (clearing it).
    pub(crate) fn take_clipboard(&self) -> Option<String> {
        self.events.clipboard.lock().unwrap().take()
    }

    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        self.osc7.feed(bytes);
        // Scheme queries answer from the ACTIVE theme, like every OSC 10/11
        // reply — one source of truth for what we'd paint.
        let replies = self.scheme.feed(bytes, crew_theme::theme().dark);
        if !replies.is_empty() {
            self.events.replies.lock().unwrap().push_str(&replies);
        }
        self.parser.advance(&mut self.term, bytes);
    }

    /// Whether the program enabled DECSET 2031 (wants scheme-change reports).
    pub(crate) fn scheme_notify_enabled(&self) -> bool {
        self.scheme.enabled()
    }

    pub(crate) fn resize(&mut self, size: GridSize) {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        self.term.resize(dims);
    }

    pub(crate) fn scroll(&mut self, delta: i32) {
        self.term.scroll_display(Scroll::Delta(delta));
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    pub(crate) fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    /// Every line the viewport can reach: the scrollback history plus the live
    /// screen. The denominator of a scroll indicator — `display_offset` alone
    /// says how far back you are, never how far back there is.
    pub(crate) fn scrollable_lines(&self) -> usize {
        let g = self.term.grid();
        g.history_size() + g.screen_lines()
    }

    pub(crate) fn bracketed_paste(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }

    /// Snapshot the DEC private modes that govern how a scroll wheel is routed.
    pub(crate) fn input_modes(&self) -> crate::modes::InputModes {
        let m = self.term.mode();
        crate::modes::InputModes {
            alt_screen: m.contains(TermMode::ALT_SCREEN),
            mouse: m.intersects(TermMode::MOUSE_MODE),
            sgr_mouse: m.contains(TermMode::SGR_MOUSE),
            app_cursor: m.contains(TermMode::APP_CURSOR),
            alternate_scroll: m.contains(TermMode::ALTERNATE_SCROLL),
        }
    }

    /// Take a pending bell (rung since the last check), clearing it.
    pub(crate) fn take_bell(&self) -> bool {
        self.events.bell.swap(false, Ordering::Relaxed)
    }

    /// Take the query replies accumulated while feeding (OSC 10/11 color
    /// queries, DSR cursor reports, …) — the bytes the child expects written
    /// back on the pty. `None` when nothing is pending.
    pub(crate) fn take_replies(&self) -> Option<String> {
        self.events.take_replies()
    }
}

#[path = "headless.rs"]
mod headless;
#[path = "modelcells.rs"]
mod modelcells;
#[path = "modelsel.rs"]
mod modelsel;
pub use headless::HeadlessTerm;

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;
