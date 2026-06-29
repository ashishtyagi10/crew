use std::sync::atomic::Ordering;

use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;

/// Background painted over selected cells.
const SELECTION_BG: (u8, u8, u8) = (54, 84, 130);

use crate::color::{resolve_color, DEFAULT_BG, DEFAULT_FG};
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
        }
    }

    /// The current program-set window title (empty if none).
    pub(crate) fn title(&self) -> String {
        self.events.title.lock().unwrap().clone()
    }

    /// Take any pending OSC 52 clipboard-store text (clearing it).
    pub(crate) fn take_clipboard(&self) -> Option<String> {
        self.events.clipboard.lock().unwrap().take()
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
        let selection = content.selection;
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
                // Selected cells take the selection background, drawn over any
                // program colours (the copied text comes from the engine).
                if selection.is_some_and(|r| r.contains(ind.point)) {
                    bg = SELECTION_BG;
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

    /// Map a viewport cell (0-based from the top-left of the visible area) to a
    /// grid `Point`, inverting the display offset that `cells()` applies — so a
    /// selection lines up while scrolled back into history. Clamped to the grid.
    fn viewport_point(&self, col: u16, row: u16) -> Point {
        let grid = self.term.grid();
        let off = grid.display_offset() as i32;
        let last_col = grid.columns().saturating_sub(1);
        let last_row = grid.screen_lines().saturating_sub(1) as u16;
        Point::new(
            Line(row.min(last_row) as i32 - off),
            Column((col as usize).min(last_col)),
        )
    }

    /// Begin a selection at viewport cell (col, row). `block` selects a
    /// rectangular column range rather than a linear character range.
    pub(crate) fn sel_start(&mut self, col: u16, row: u16, block: bool) {
        let point = self.viewport_point(col, row);
        let ty = if block {
            SelectionType::Block
        } else {
            SelectionType::Simple
        };
        self.term.selection = Some(Selection::new(ty, point, Side::Left));
    }

    /// Extend the active selection's end to viewport cell (col, row). The end
    /// cell is inclusive (Side::Right) so the cell under the cursor is selected.
    pub(crate) fn sel_update(&mut self, col: u16, row: u16) {
        let point = self.viewport_point(col, row);
        if let Some(sel) = self.term.selection.as_mut() {
            sel.update(point, Side::Right);
        }
    }

    pub(crate) fn sel_clear(&mut self) {
        self.term.selection = None;
    }

    /// The selected text, or `None` when there's no (non-empty) selection.
    pub(crate) fn sel_text(&self) -> Option<String> {
        self.term.selection_to_string().filter(|s| !s.is_empty())
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

    pub(crate) fn bracketed_paste(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }

    /// Take a pending bell (rung since the last check), clearing it.
    pub(crate) fn take_bell(&self) -> bool {
        self.events.bell.swap(false, Ordering::Relaxed)
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

    pub fn scroll(&mut self, delta: i32) {
        self.core.scroll(delta);
    }

    pub fn display_offset(&self) -> usize {
        self.core.display_offset()
    }

    pub fn title(&self) -> String {
        self.core.title()
    }

    pub fn take_bell(&self) -> bool {
        self.core.take_bell()
    }

    pub fn take_clipboard(&self) -> Option<String> {
        self.core.take_clipboard()
    }
}

impl HeadlessTerm {
    pub fn sel_start(&mut self, col: u16, row: u16, block: bool) {
        self.core.sel_start(col, row, block);
    }

    pub fn sel_update(&mut self, col: u16, row: u16) {
        self.core.sel_update(col, row);
    }

    pub fn sel_clear(&mut self) {
        self.core.sel_clear();
    }

    pub fn sel_text(&self) -> Option<String> {
        self.core.sel_text()
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

#[cfg(test)]
mod selection_tests {
    use super::{GridSize, HeadlessTerm, TermModel};

    fn term(text: &str) -> HeadlessTerm {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(text.as_bytes());
        t
    }

    #[test]
    fn no_selection_yields_no_text() {
        assert_eq!(term("hello").sel_text(), None);
    }

    #[test]
    fn drag_selects_an_inclusive_character_span() {
        let mut t = term("hello world");
        // Drag from column 0 to column 4 on row 0 — the cell under the cursor is
        // included, so this is "hello", not "hell".
        t.sel_start(0, 0, false);
        t.sel_update(4, 0);
        assert_eq!(t.sel_text().as_deref(), Some("hello"));
    }

    #[test]
    fn clearing_drops_the_selection() {
        let mut t = term("hello");
        t.sel_start(0, 0, false);
        t.sel_update(4, 0);
        t.sel_clear();
        assert_eq!(t.sel_text(), None);
    }

    #[test]
    fn selected_cells_render_with_the_selection_background() {
        let mut t = term("hello");
        // Select "he" (columns 0..=1 on row 0).
        t.sel_start(0, 0, false);
        t.sel_update(1, 0);
        let cells = t.cells(false);
        let bg = |ch| cells.iter().find(|c| c.c == ch).map(|c| c.bg);
        assert_eq!(bg('h'), Some(super::SELECTION_BG));
        assert_eq!(bg('e'), Some(super::SELECTION_BG));
        // 'o' is outside the selection — it keeps the normal background.
        assert_ne!(bg('o'), Some(super::SELECTION_BG));
    }

    #[test]
    fn block_selection_takes_a_column_range_across_rows() {
        let mut t = term("abcde\r\nABCDE");
        // Rectangular columns 1..=3 over rows 0..=1 → "bcd" and "BCD".
        t.sel_start(1, 0, true);
        t.sel_update(3, 1);
        let txt = t.sel_text().unwrap_or_default();
        assert!(txt.contains("bcd") && txt.contains("BCD"), "got {txt:?}");
    }
}
