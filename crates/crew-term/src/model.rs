use std::sync::atomic::Ordering;

use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;

/// Background painted over selected cells.
const SELECTION_BG: (u8, u8, u8) = (54, 84, 130);

/// A desaturated (grey) background at either extreme — the kind agent CLIs
/// paint behind the line you just sent, tuned to whichever theme they detected
/// at startup: dark grey on a dark guess (`ESC[48;2;55;55;55m`), light grey on
/// a light one (`ESC[48;2;230;230;230m`). After a live theme switch the
/// opposite-theme variant lands as a glaring block (white word-boxes on the
/// dark canvas). The `≤24` channel spread keeps the match grey-only, and the
/// dark `≤96` / light `≥160` bounds leave mid-greys and every saturated
/// background that carries meaning (diff red/green, error rows) untouched.
fn is_echo_grey((r, g, b): (u8, u8, u8)) -> bool {
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    (mx <= 96 || mn >= 160) && mx - mn <= 24
}

/// Whether a program-painted background should be dropped to the canvas.
///
/// In a dark theme, agent CLIs paint "highlight" backgrounds tuned to
/// whichever theme they detected at startup — dark grey, light grey, or
/// (after a live switch) the opposite-theme variant — and any of those reads
/// as an ugly box on the flat dark canvas regardless of how close to the
/// extremes it sits. So in dark mode we drop any low-saturation (`≤24`
/// channel spread) background, MID-grey included, plus any background bright
/// enough to itself read as a light "highlight" box (luminance `> 0.6`) even
/// if it happens to carry a little saturation. Saturated *dark* colours
/// (spread `> 24` and luminance `≤ 0.6` — diff red/green, error rows) survive
/// untouched.
///
/// In a light theme the existing (dark/light-extreme-only) echo-grey
/// behaviour is unchanged.
fn should_drop_bg((r, g, b): (u8, u8, u8), dark: bool) -> bool {
    if dark {
        let mx = r.max(g).max(b);
        let mn = r.min(g).min(b);
        (mx - mn <= 24) || crate::contrast::luminance((r, g, b)) > 0.6
    } else {
        is_echo_grey((r, g, b))
    }
}

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
        let dark = crate::contrast::luminance(default_bg()) < 0.5;
        let mut out: Vec<RenderCell> = content
            .display_iter
            .filter(|ind| ind.c != ' ' && ind.c != '\0' && ind.point.line.0 + off >= 0)
            .map(|ind| {
                let bold = ind.flags.contains(Flags::BOLD);
                let italic = ind.flags.contains(Flags::ITALIC);
                let fg = resolve_color(ind.fg, palette, default_fg());
                let mut bg = resolve_color(ind.bg, palette, default_bg());
                // Reverse-video (SGR 7) is intentionally NOT honoured: programs
                // (e.g. agent CLIs) use it to "highlight" the line you just sent,
                // which renders as a hard-to-read block. Dropping the fg/bg swap
                // shows that text plainly instead.
                // Agent CLIs (Claude/codex) also paint the just-sent line with a
                // real near-grey background tuned to the theme they detected at
                // startup — dark grey on dark, light grey on light — which reads
                // as a muddy (or glaring) block on the actual canvas. Drop those
                // echo greys so the text shows plainly, while keeping saturated
                // backgrounds that carry meaning (diffs, errors). In a dark
                // theme this also flattens MID-grey and any light "highlight"
                // background (see `should_drop_bg`), since the flat-canvas
                // vision is stricter there than the light-theme extremes-only
                // check.
                if should_drop_bg(bg, dark) {
                    bg = default_bg();
                }
                // Selected cells take the selection background, drawn over any
                // program colours (the copied text comes from the engine).
                if selection.is_some_and(|r| r.contains(ind.point)) {
                    bg = SELECTION_BG;
                }
                // Legibility floor: a program that sampled the background once
                // (or guessed wrong) keeps painting for the other theme after a
                // live switch — nudge any too-close fg until it reads.
                let fg = crate::contrast::ensure_min_contrast(fg, bg);
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
        self.sel_anchor = Some((point, ty));
        self.term.selection = Some(Selection::new(ty, point, Side::Left));
    }

    /// Extend the active selection's end to viewport cell (col, row), keeping
    /// both end cells inclusive whichever way the drag runs.
    ///
    /// The sides cannot be fixed: `to_range` swaps the anchors when the drag
    /// runs backwards but KEEPS their sides, then trims the last cell if
    /// `end.side == Left` and the first if `start.side == Right`. With a
    /// hard-coded (Left, Right) pair a backward drag swapped to (Right, Left)
    /// and lost a character off EACH end — dragging right-to-left across
    /// "hello" copied "ell". Alacritty itself avoids this by deriving the side
    /// from where in the cell the pointer sits; we only have whole cells, so
    /// derive it from the direction instead: the pair must come out (Left,
    /// Right) *after* any swap.
    pub(crate) fn sel_update(&mut self, col: u16, row: u16) {
        let point = self.viewport_point(col, row);
        let Some((anchor, ty)) = self.sel_anchor else {
            return;
        };
        let (anchor_side, cursor_side) = if point < anchor {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        };
        let mut sel = Selection::new(ty, anchor, anchor_side);
        sel.update(point, cursor_side);
        self.term.selection = Some(sel);
    }

    pub(crate) fn sel_clear(&mut self) {
        self.term.selection = None;
        self.sel_anchor = None;
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

    pub fn take_cwd(&mut self) -> Option<std::path::PathBuf> {
        self.core.take_cwd()
    }

    pub fn take_bell(&self) -> bool {
        self.core.take_bell()
    }

    pub fn take_clipboard(&self) -> Option<String> {
        self.core.take_clipboard()
    }

    /// Take pending query replies (OSC color / DSR reports) owed to the child.
    pub fn take_replies(&self) -> Option<String> {
        self.core.take_replies()
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
mod reply_tests {
    use super::{GridSize, HeadlessTerm, TermModel};

    /// OSC 11 (background query) must be answered from the active theme — agent
    /// CLIs (claude, codex) probe it to pick a light or dark output palette;
    /// unanswered, they assume dark and paint light text onto light themes.
    #[test]
    fn osc11_background_query_is_answered_from_theme() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(b"\x1b]11;?\x1b\\");
        let reply = t.take_replies().expect("background query answered");
        let (r, g, b) = crew_theme::theme().term_bg;
        assert_eq!(
            reply,
            format!("\x1b]11;rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\x1b\\")
        );
        assert_eq!(t.take_replies(), None, "replies drain on take");
    }

    /// OSC 10 (foreground query) answers with the theme's terminal foreground.
    #[test]
    fn osc10_foreground_query_is_answered_from_theme() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(b"\x1b]10;?\x1b\\");
        let reply = t.take_replies().expect("foreground query answered");
        let (r, _, _) = crew_theme::theme().term_fg;
        assert!(reply.starts_with("\x1b]10;rgb:"), "{reply:?}");
        assert!(reply.contains(&format!("{r:02x}{r:02x}/")), "{reply:?}");
    }

    /// DSR 6 (cursor position report) flows back through `PtyWrite`.
    #[test]
    fn dsr_cursor_position_is_reported() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(b"ab\x1b[6n");
        assert_eq!(t.take_replies().as_deref(), Some("\x1b[1;3R"));
    }

    /// A program painting truecolor text at (or near) the terminal background —
    /// a dark-theme palette left running across a live switch to a light theme,
    /// say — must still render legibly: the contrast floor nudges the fg.
    #[test]
    fn near_background_truecolor_text_stays_legible() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        let (r, g, b) = crew_theme::theme().term_bg;
        t.feed(format!("\x1b[38;2;{r};{g};{b}mhi").as_bytes());
        let cells = t.cells(false);
        let h = cells.iter().find(|c| c.c == 'h').expect("cell rendered");
        assert!(
            crate::contrast::ratio(h.fg, h.bg) >= crate::contrast::MIN_CONTRAST - 0.1,
            "bg-on-bg text rendered at ratio {} (fg {:?} on bg {:?})",
            crate::contrast::ratio(h.fg, h.bg),
            h.fg,
            h.bg
        );
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
    fn inverse_video_is_not_drawn_as_a_highlight() {
        // 'X' is plain; 'H' is reverse-video (SGR 7). With the program's
        // highlight suppressed, the inverse cell must render with the SAME
        // colours as the plain one — no swapped fg/bg "highlight" block.
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
        t.feed(b"X\x1b[7mH\x1b[0m");
        let cells = t.cells(false);
        let x = cells.iter().find(|c| c.c == 'X').expect("X rendered");
        let h = cells.iter().find(|c| c.c == 'H').expect("H rendered");
        assert_eq!(h.fg, x.fg, "inverse cell should keep the normal foreground");
        assert_eq!(h.bg, x.bg, "inverse cell should keep the normal background");
    }

    #[test]
    fn dim_grey_echo_background_is_dropped() {
        // Agent CLIs paint the just-sent line with a dark-grey background
        // (ESC[48;2;55;55;55m). 'X' is plain; 'H' carries that grey bg — which
        // must be dropped so it renders on the same canvas as the plain cell.
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
        t.feed(b"X\x1b[48;2;55;55;55mH\x1b[0m");
        let cells = t.cells(false);
        let x = cells.iter().find(|c| c.c == 'X').expect("X rendered");
        let h = cells.iter().find(|c| c.c == 'H').expect("H rendered");
        assert_eq!(h.bg, x.bg, "dark-grey echo background should be dropped");
    }

    #[test]
    fn mid_grey_program_background_is_flattened_in_dark_theme() {
        // The regression `is_echo_grey` missed: a MID-grey highlight (neither
        // near-black nor near-white) still reads as an ugly box on the flat
        // dark canvas and must be dropped too. Only meaningful if the test
        // theme is dark (default is PaperDark — see crew-theme's
        // `default_is_paper_dark`); the pure `should_drop_bg` tests below are
        // the primary coverage regardless of theme.
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
        t.feed(b"X\x1b[48;2;140;140;140mH\x1b[0m");
        let cells = t.cells(false);
        let x = cells.iter().find(|c| c.c == 'X').expect("X rendered");
        let h = cells.iter().find(|c| c.c == 'H').expect("H rendered");
        if crew_theme::theme().term_bg == crew_theme::PAPER_DARK.term_bg {
            assert_eq!(
                h.bg, x.bg,
                "mid-grey program background should be flattened in a dark theme"
            );
        }
    }

    #[test]
    fn light_grey_echo_background_is_dropped() {
        // The same echo highlight painted for the OPPOSITE theme: a CLI that
        // detected a light background (or outlived a live switch to dark)
        // paints the just-sent line light-grey (ESC[48;2;230;230;230m). On the
        // dark canvas that reads as white word-boxes — drop it like the dark
        // variant.
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
        t.feed(b"X\x1b[48;2;230;230;230mH\x1b[0m");
        let cells = t.cells(false);
        let x = cells.iter().find(|c| c.c == 'X').expect("X rendered");
        let h = cells.iter().find(|c| c.c == 'H').expect("H rendered");
        assert_eq!(h.bg, x.bg, "light-grey echo background should be dropped");
    }

    #[test]
    fn saturated_dark_background_is_kept() {
        // A dark-but-coloured background (e.g. a diff's green) carries meaning and
        // must survive — only desaturated greys are treated as echo highlights.
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
        t.feed(b"\x1b[48;2;0;60;0mD\x1b[0m");
        let cells = t.cells(false);
        let d = cells.iter().find(|c| c.c == 'D').expect("D rendered");
        assert_eq!(d.bg, (0, 60, 0), "saturated dark background should be kept");
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

    /// The same span, dragged the other way, must copy the same text.
    ///
    /// `sel_start` hard-coded `Side::Left` and `sel_update` `Side::Right`,
    /// which is only right for a FORWARD drag. On a reverse drag alacritty's
    /// `to_range` swaps the anchors but keeps their sides, then trims the last
    /// cell when `end.side == Left` and the first when `start.side == Right` —
    /// so a right-to-left drag over "hello" copied "ell". The suite only ever
    /// dragged left-to-right, so it never saw it.
    #[test]
    fn a_backward_drag_selects_the_same_span_as_a_forward_one() {
        let forward = {
            let mut t = term("hello world");
            t.sel_start(0, 0, false);
            t.sel_update(4, 0);
            t.sel_text()
        };
        let mut t = term("hello world");
        t.sel_start(4, 0, false); // press on the 'o'
        t.sel_update(0, 0); // drag back to the 'h'
        assert_eq!(
            t.sel_text().as_deref(),
            Some("hello"),
            "a right-to-left drag lost characters"
        );
        assert_eq!(t.sel_text(), forward, "drag direction changed the text");
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

#[cfg(test)]
mod should_drop_bg_tests {
    use super::should_drop_bg;

    #[test]
    fn dark_theme_drops_mid_grey() {
        // The regression the old `is_echo_grey` missed: a MID-grey highlight
        // (neither near-black nor near-white) reads just as ugly on a flat
        // dark canvas as the extremes did.
        assert!(should_drop_bg((140, 140, 140), true));
    }

    #[test]
    fn dark_theme_drops_light_grey() {
        assert!(should_drop_bg((230, 230, 230), true));
    }

    #[test]
    fn dark_theme_keeps_saturated_diff_green() {
        assert!(!should_drop_bg((30, 110, 50), true));
    }

    #[test]
    fn dark_theme_keeps_saturated_diff_red() {
        assert!(!should_drop_bg((110, 40, 45), true));
    }

    #[test]
    fn light_theme_keeps_mid_grey() {
        // Light-theme behaviour is unchanged: `is_echo_grey`'s extremes-only
        // check does not treat mid-grey as an echo highlight.
        assert!(!should_drop_bg((140, 140, 140), false));
    }
}
