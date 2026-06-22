use std::sync::{Arc, Mutex};

use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;

use crate::color::{resolve_color, DEFAULT_BG, DEFAULT_FG};
use crate::listener::TitleListener;

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
    term: Term<TitleListener>,
    parser: Processor,
    title: Arc<Mutex<String>>,
}

impl TermCore {
    pub(crate) fn new(size: GridSize) -> Self {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        let title = Arc::new(Mutex::new(String::new()));
        let listener = TitleListener {
            title: title.clone(),
        };
        let term = Term::new(Config::default(), &dims, listener);
        Self {
            term,
            parser: Processor::new(),
            title,
        }
    }

    /// The current program-set window title (empty if none).
    pub(crate) fn title(&self) -> String {
        self.title.lock().unwrap().clone()
    }

    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    pub(crate) fn cells(&self, focused: bool) -> Vec<RenderCell> {
        let content = self.term.renderable_content();
        let palette = content.colors;
        // When scrolled into history, viewport lines are negative; add the display
        // offset to map each line back to a 0-based viewport row.
        let off = content.display_offset as i32;
        let cursor = content.cursor;
        let mut out: Vec<RenderCell> = content
            .display_iter
            .filter(|ind| ind.c != ' ' && ind.c != '\0' && ind.point.line.0 + off >= 0)
            .map(|ind| {
                let bold = ind.flags.contains(Flags::BOLD);
                let italic = ind.flags.contains(Flags::ITALIC);
                let mut fg = resolve_color(ind.fg, palette, DEFAULT_FG);
                let mut bg = resolve_color(ind.bg, palette, DEFAULT_BG);
                if ind.flags.contains(Flags::INVERSE) {
                    std::mem::swap(&mut fg, &mut bg);
                }
                RenderCell {
                    col: ind.point.column.0 as u16,
                    row: (ind.point.line.0 + off) as u16,
                    c: ind.c,
                    fg,
                    bg,
                    bold,
                    italic,
                }
            })
            .collect();
        crate::cursor::apply(&mut out, &cursor, off, focused);
        out
    }

    pub(crate) fn resize(&mut self, size: GridSize) {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        self.term.resize(dims);
    }

    /// Scroll the viewport by `delta` lines into scrollback (positive = older).
    pub(crate) fn scroll(&mut self, delta: i32) {
        self.term.scroll_display(Scroll::Delta(delta));
    }

    /// Jump back to the live bottom of the terminal.
    pub(crate) fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    /// Lines currently scrolled back from the live bottom (0 = at the bottom).
    pub(crate) fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    /// Whether the program enabled bracketed-paste mode.
    pub(crate) fn bracketed_paste(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }
}

pub struct HeadlessTerm {
    core: TermCore,
}

impl HeadlessTerm {
    pub fn new(size: GridSize) -> Self {
        Self {
            core: TermCore::new(size),
        }
    }

    /// Scroll the viewport by `delta` lines into scrollback (positive = older).
    pub fn scroll(&mut self, delta: i32) {
        self.core.scroll(delta);
    }

    /// Lines currently scrolled back from the live bottom (0 = at the bottom).
    pub fn display_offset(&self) -> usize {
        self.core.display_offset()
    }

    /// The program-set window title (empty if none).
    pub fn title(&self) -> String {
        self.core.title()
    }
}

impl TermModel for HeadlessTerm {
    fn feed(&mut self, bytes: &[u8]) {
        self.core.feed(bytes);
    }

    fn cells(&self, focused: bool) -> Vec<RenderCell> {
        self.core.cells(focused)
    }

    fn resize(&mut self, size: GridSize) {
        self.core.resize(size);
    }
}
