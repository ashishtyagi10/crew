use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;

use crate::color::{resolve_color, DEFAULT_BG, DEFAULT_FG};

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
    fn cells(&self) -> Vec<RenderCell>;
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

// A no-op event listener — we don't react to terminal events yet.
#[derive(Clone)]
struct NoopListener;

impl EventListener for NoopListener {
    fn send_event(&self, _event: Event) {}
}

// Shared core: a Term + an ANSI processor. Used by HeadlessTerm and PtyTerm.
pub(crate) struct TermCore {
    term: Term<NoopListener>,
    parser: Processor,
}

impl TermCore {
    pub(crate) fn new(size: GridSize) -> Self {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        let term = Term::new(Config::default(), &dims, NoopListener);
        Self {
            term,
            parser: Processor::new(),
        }
    }

    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    pub(crate) fn cells(&self) -> Vec<RenderCell> {
        let content = self.term.renderable_content();
        let palette = content.colors;
        // display_iter yields Indexed<&Cell>; Indexed derefs to Cell, so .c is available.
        // point.line is i32 (0 = top of viewport); point.column is usize.
        content
            .display_iter
            .filter(|ind| ind.c != ' ' && ind.c != '\0' && ind.point.line.0 >= 0)
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
                    row: ind.point.line.0 as u16,
                    c: ind.c,
                    fg,
                    bg,
                    bold,
                    italic,
                }
            })
            .collect()
    }

    pub(crate) fn resize(&mut self, size: GridSize) {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        self.term.resize(dims);
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
}

impl TermModel for HeadlessTerm {
    fn feed(&mut self, bytes: &[u8]) {
        self.core.feed(bytes);
    }

    fn cells(&self) -> Vec<RenderCell> {
        self.core.cells()
    }

    fn resize(&mut self, size: GridSize) {
        self.core.resize(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A headless Term (no PTY) we can feed bytes into deterministically.
    #[test]
    fn feeding_text_appears_in_cells() {
        let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 5 });
        term.feed(b"hi");
        let cells = term.cells();
        let text: String = {
            let mut row0: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
            row0.sort_by_key(|c| c.col);
            row0.iter().map(|c| c.c).collect()
        };
        assert_eq!(text, "hi");
    }

    #[test]
    fn sgr_red_bold_is_resolved_to_rgb_and_flags() {
        let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
        // ESC[1m bold, ESC[31m red foreground, then "X"
        term.feed(b"\x1b[1m\x1b[31mX");
        let cell = term
            .cells()
            .into_iter()
            .find(|c| c.c == 'X')
            .expect("cell X");
        assert!(cell.bold, "bold flag should be set");
        // Default ANSI red has a high red channel and low green/blue.
        assert!(
            cell.fg.0 > 120 && cell.fg.1 < 100 && cell.fg.2 < 100,
            "fg should be reddish, got {:?}",
            cell.fg
        );
    }
}
