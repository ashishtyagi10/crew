use std::sync::atomic::Ordering;

use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Osc52, Term, TermMode};
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
    /// Sniffs OSC 22 (the pointer shape a program asks for), invisible to the parser.
    pointer: crate::pointershape::PointerShape,
    /// DECRQSS, answered from the state at the request (`modelrqss`).
    rqss: modelrqss::Rqss,
    /// Answers `CSI 16 t`, XTVERSION and XTGETTCAP, which the parser does not dispatch.
    cellq: modelsize::Queries,
    /// Where the drag began, and its kind. Kept because a selection's sides
    /// depend on the drag's DIRECTION, which only the anchor can tell us, and
    /// `Selection` doesn't hand its anchor back. See `sel_update`.
    sel_anchor: Option<(Point, SelectionType)>,
    /// The splitter that lifts graphics sequences out of the byte stream.
    graphics: crate::graphics::GraphicsScanner,
    /// Pictures placed since the app last collected them.
    images: Vec<PlacedImage>,
    /// Watches for the erases and screen switches that drop pictures.
    wipes: modelwipe::Wipes,
    /// The frame's cell size in pixels: the rows a picture reserves, and the
    /// size reports (`modelsize`). The app publishes it on every relayout; a
    /// plausible 8×16 until then.
    cell_px: (u32, u32),
    /// Lines thrown off the top of a full scrollback, ever: what keeps a
    /// picture's absolute anchor moving once `history_size` stops growing
    /// (see `modelevict`).
    evicted: u64,
    /// How many lines of scrollback the primary screen keeps.
    history_cap: usize,
    /// Whether a focused pane draws its cursor this frame: false only in the
    /// dark half of a blink, which the app times (see `cursorblink`).
    cursor_lit: bool,
}

/// A picture the program asked for, and where in the buffer it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedImage {
    /// Absolute buffer line: every line ever scrolled into the history (kept
    /// or since evicted) plus the cursor's row. Stable as the screen scrolls,
    /// which a viewport row is not; `TermCore::screen_line` is the top row's.
    pub line: u64,
    pub col: u16,
    /// How many columns and rows of the grid it covers.
    pub cells: (u16, u16),
    /// Placed on the alternate screen: shown only there (see `wipescan`).
    pub alt: bool,
    pub cmd: crate::graphicscmd::ImageCmd,
}

impl TermCore {
    pub(crate) fn new(size: GridSize) -> Self {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        let events = TermEvents::default();
        // A cursor nobody styled stays steady (crew's look before blink came, and an idle
        // prompt then costs no frames); a program asks to blink with 1/3/5 or `?12h`.
        // The kitty keyboard protocol is honoured (see `modelkitty`). OSC 52 reads are
        // parsed always and gated by the app when it answers (see `modelclip`).
        let config = Config {
            kitty_keyboard: true,
            osc52: Osc52::CopyPaste,
            ..Config::default()
        };
        let history_cap = config.scrolling_history;
        let term = Term::new(config, &dims, events.clone());
        let core = Self {
            term,
            parser: Processor::new(),
            events,
            osc7: crate::osc::OscScanner::default(),
            scheme: crate::schemenotify::SchemeNotify::default(),
            pointer: Default::default(),
            rqss: Default::default(),
            sel_anchor: None,
            graphics: crate::graphics::GraphicsScanner::default(),
            images: Vec::new(),
            wipes: Default::default(),
            cell_px: (8, 16),
            cellq: modelsize::Queries::default(),
            evicted: 0,
            history_cap,
            cursor_lit: true,
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

    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        let prompts = self.osc7.feed(bytes);
        self.feed_split(bytes, &prompts);
    }

    /// Everything but the OSC sniff and DECRQSS, for one piece of a chunk
    /// (`feed_split` cuts a chunk at each prompt mark, `modelrqss` at each
    /// DECRQSS).
    fn feed_piece(&mut self, bytes: &[u8]) {
        // Scheme queries answer from the ACTIVE theme, like OSC 10/11.
        self.pointer.feed(bytes);
        let was_alt = self
            .term
            .mode()
            .contains(alacritty_terminal::term::TermMode::ALT_SCREEN);
        let mut replies = self.scheme.feed(bytes, crew_theme::theme().dark);
        replies.push_str(&self.query_replies(bytes));
        if !replies.is_empty() {
            self.events.replies.lock().unwrap().push_str(&replies);
        }
        // Pictures are split OUT of the stream rather than parsed from it:
        // an image lands where the cursor is, so the bytes before it must
        // reach the parser first (see `crate::graphics`).
        let mut scanner = std::mem::take(&mut self.graphics);
        for seg in scanner.feed(bytes) {
            match seg {
                crate::graphics::Seg::Bytes(b) => self.advance(b),
                crate::graphics::Seg::Held(b) => self.advance(&b),
                crate::graphics::Seg::Image(cmd) => self.place_image(cmd),
            }
        }
        self.graphics = scanner;
        // A TUI that leaves the alternate screen takes its hand with it, even
        // when no shell mark is ever seen (ssh, tmux, no integration).
        if was_alt && !self.input_modes().alt_screen {
            self.pointer.clear();
        }
    }

    /// Publish the frame's cell size (pictures' rows, size reports).
    pub(crate) fn set_cell_px(&mut self, w: u32, h: u32) {
        self.cell_px = (w.max(1), h.max(1));
        self.publish_size();
    }

    /// Whether the program enabled DECSET 2031 (wants scheme-change reports).
    pub(crate) fn scheme_notify_enabled(&self) -> bool {
        self.scheme.enabled()
    }

    /// The OSC 22 pointer shape the program asked for, if any.
    pub(crate) fn pointer_shape(&self) -> Option<&str> {
        self.pointer.name()
    }

    pub(crate) fn resize(&mut self, size: GridSize) {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        self.term.resize(dims);
        self.publish_size();
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

#[path = "cursorblink.rs"]
mod cursorblink;
#[path = "cursorpos.rs"]
mod cursorpos;
#[path = "headless.rs"]
mod headless;
#[path = "modelcells.rs"]
mod modelcells;
#[path = "modelclip.rs"]
mod modelclip;
#[path = "modelcolor.rs"]
mod modelcolor;
#[path = "modelevict.rs"]
mod modelevict;
#[path = "modelfind.rs"]
mod modelfind;
pub use modelfind::{FindSpan, FIND_CAP};
#[path = "modelimage.rs"]
mod modelimage;
#[path = "modellast.rs"]
mod modellast;
#[path = "modelmodes.rs"]
mod modelmodes;
#[path = "modelprompt.rs"]
mod modelprompt;
#[path = "modelrqss.rs"]
mod modelrqss;
#[path = "modelrqsssgr.rs"]
mod modelrqsssgr;
#[path = "modelscroll.rs"]
mod modelscroll;
#[path = "modelsel.rs"]
mod modelsel;
#[path = "modelsize.rs"]
mod modelsize;
#[path = "modelsync.rs"]
mod modelsync;
#[path = "modelwipe.rs"]
mod modelwipe;
#[path = "modelwrap.rs"]
mod modelwrap;
pub use headless::HeadlessTerm;
pub use modelclip::ClipboardAsk;

#[cfg(test)]
#[path = "graphicsplace_tests.rs"]
mod graphicsplace_tests;
#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;
#[cfg(test)]
#[path = "modelmarks_tests.rs"]
mod modelmarks_tests;
#[cfg(test)]
#[path = "modelprompt_tests.rs"]
mod modelprompt_tests;
