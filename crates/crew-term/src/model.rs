use std::sync::atomic::Ordering;

use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;

use crate::color::{default_bg, resolve_color};
use crate::listener::TermEvents;

#[derive(Clone, Copy, Debug)]
pub struct GridSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderCell {
    pub col: u16,
    pub row: u16,
    pub c: char,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
    pub bold: bool,
    pub italic: bool,
    /// What the grid says this cell wears beyond its glyph (SGR 4/9/58).
    pub deco: crew_theme::deco::Deco,
    /// The cursor, when this is the cell it sits on.
    pub cursor: crew_theme::deco::CursorMark,
    /// The combining marks alacritty keeps after the cell's character
    /// (`Cell::zerowidth`): a virama, a nasal dot, an accent.
    pub marks: crew_theme::marks::Marks,
}

pub trait TermModel {
    fn feed(&mut self, bytes: &[u8]);
    /// Render cells; `focused` brightens the block cursor (dim otherwise).
    fn cells(&self, focused: bool) -> Vec<RenderCell>;
    /// The bottom-most viewport row with anything on it (see `modellast`) —
    /// one line of text without building the screen.
    fn last_line(&self) -> Option<String>;
    /// Per viewport row, whether it soft-wraps onto the next (see `modelwrap`).
    fn wrapped_rows(&self) -> Vec<bool>;
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
    osc7: crate::osc::OscScanner,
    /// Sniffs DECSET 2031 (color-scheme notifications), invisible to the parser.
    scheme: crate::schemenotify::SchemeNotify,
    /// Answers `CSI 16 t`, which the parser does not dispatch (`modelsize`).
    cellq: modelsize::CellQuery,
    /// Where the drag began, and its kind. Kept because a selection's sides
    /// depend on the drag's DIRECTION, which only the anchor can tell us, and
    /// `Selection` doesn't hand its anchor back. See `sel_update`.
    sel_anchor: Option<(Point, SelectionType)>,
    /// The splitter that lifts graphics sequences out of the byte stream.
    graphics: crate::graphics::GraphicsScanner,
    /// Pictures placed since the app last collected them.
    images: Vec<PlacedImage>,
    /// The frame's cell size in pixels: the rows a picture reserves, and the
    /// size reports (`modelsize`). The app publishes it on every relayout; a
    /// plausible 8×16 until then.
    cell_px: (u32, u32),
}

/// A picture the program asked for, and where in the buffer it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedImage {
    /// Absolute buffer line: scrollback above the screen plus the cursor's
    /// row. Stable as the screen scrolls, which a viewport row is not.
    pub line: u64,
    pub col: u16,
    /// How many columns and rows of the grid it covers.
    pub cells: (u16, u16),
    pub cmd: crate::graphicscmd::ImageCmd,
}

impl TermCore {
    pub(crate) fn new(size: GridSize) -> Self {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        let events = TermEvents::default();
        let term = Term::new(Config::default(), &dims, events.clone());
        let core = Self {
            term,
            parser: Processor::new(),
            events,
            osc7: crate::osc::OscScanner::default(),
            scheme: crate::schemenotify::SchemeNotify::default(),
            sel_anchor: None,
            graphics: crate::graphics::GraphicsScanner::default(),
            images: Vec::new(),
            cell_px: (8, 16),
            cellq: modelsize::CellQuery::default(),
        };
        core.publish_size();
        core
    }

    /// The current program-set window title (empty if none).
    pub(crate) fn title(&self) -> String {
        self.events.title.lock().unwrap().clone()
    }

    /// The directory reported by the program (OSC 7) if it changed since the last
    /// call, else `None`.
    pub(crate) fn take_notify(&mut self) -> Option<(String, String)> {
        self.osc7.take_notify()
    }

    pub(crate) fn progress(&self) -> Option<crate::osc::Progress> {
        self.osc7.progress()
    }

    pub(crate) fn take_cwd(&mut self) -> Option<std::path::PathBuf> {
        self.osc7.take_cwd()
    }

    pub(crate) fn take_shell(&mut self) -> Vec<crate::osc::ShellMark> {
        self.osc7.take_shell()
    }

    /// Take any pending OSC 52 clipboard-store text (clearing it).
    pub(crate) fn take_clipboard(&self) -> Option<String> {
        self.events.clipboard.lock().unwrap().take()
    }

    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        self.osc7.feed(bytes);
        // Scheme queries answer from the ACTIVE theme, like OSC 10/11.
        let mut replies = self.scheme.feed(bytes, crew_theme::theme().dark);
        replies.push_str(&self.cellq.feed(bytes, self.cell_px));
        if !replies.is_empty() {
            self.events.replies.lock().unwrap().push_str(&replies);
        }
        // Pictures are split OUT of the stream rather than parsed from it:
        // an image lands where the cursor is, so the bytes before it must
        // reach the parser first (see `crate::graphics`).
        let mut scanner = std::mem::take(&mut self.graphics);
        for seg in scanner.feed(bytes) {
            match seg {
                crate::graphics::Seg::Bytes(b) => self.parser.advance(&mut self.term, b),
                crate::graphics::Seg::Held(b) => self.parser.advance(&mut self.term, &b),
                crate::graphics::Seg::Image(cmd) => self.place_image(cmd),
            }
        }
        self.graphics = scanner;
    }

    /// Take the pictures placed since the last call — `crew-app` decodes them
    /// off this thread.
    pub(crate) fn take_images(&mut self) -> Vec<crate::model::PlacedImage> {
        std::mem::take(&mut self.images)
    }

    /// Publish the frame's cell size (pictures' rows, size reports).
    pub(crate) fn set_cell_px(&mut self, w: u32, h: u32) {
        self.cell_px = (w.max(1), h.max(1));
        self.publish_size();
    }

    /// Lines of scrollback above the screen, for mapping an image's absolute
    /// anchor back to the row it is on now.
    pub(crate) fn history_lines(&self) -> usize {
        self.term.grid().history_size()
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
        self.publish_size();
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

#[path = "cursorpos.rs"]
mod cursorpos;
#[path = "headless.rs"]
mod headless;
#[path = "modelcells.rs"]
mod modelcells;
#[path = "modelimage.rs"]
mod modelimage;
#[path = "modellast.rs"]
mod modellast;
#[path = "modelmodes.rs"]
mod modelmodes;
#[path = "modelsel.rs"]
mod modelsel;
#[path = "modelsize.rs"]
mod modelsize;
#[path = "modelsync.rs"]
mod modelsync;
#[path = "modelwrap.rs"]
mod modelwrap;
pub use headless::HeadlessTerm;

#[cfg(test)]
#[path = "graphicsplace_tests.rs"]
mod graphicsplace_tests;
#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;
#[cfg(test)]
#[path = "modelmarks_tests.rs"]
mod modelmarks_tests;
