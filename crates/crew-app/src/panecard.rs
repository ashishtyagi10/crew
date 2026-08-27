//! Pane fieldset drawing: the rounded border + legend that frames a pane
//! ([`pane_card`]) — signature hue, status glyphs. No title bars: a pane is
//! just a border with a legend on its top edge. Non-pane panels (sidebar,
//! welcome, menus) use [`crate::panelcard::push_card`].
//!
//! Busy panes carry no in-pane indicator here: the chat header already states
//! the running task and its elapsed time, and the nav strip pulses its dot
//! (see [`crate::minstrip`]).
use crew_render::CellView;

use crate::boxdraw::titled_card;
use crate::layout::Rect;

/// Inputs for one pane's fieldset border.
pub(crate) struct Bar<'a> {
    pub index: Option<usize>,
    pub title: &'a str,
    pub focused: bool,
    /// Lines scrolled back from the live bottom (0 = at the bottom).
    pub scroll: usize,
    /// Every line the pane's viewport can reach — scrollback plus the live
    /// screen. The denominator of the border thumb (see
    /// [`crate::panescroll`]); `scroll` alone can only say how far from the
    /// bottom you are, never how far back there is.
    pub total: usize,
    pub activity: bool,
    pub bell: bool,
    /// This pane is receiving broadcast (synchronized) input.
    pub broadcast: bool,
    /// Draw the `[-][x]` minimize and close buttons on the top border (full grid
    /// tiles and the zoomed tile — not strip thumbnails). Click regions come from
    /// [`min_btn_rect`] and [`close_btn_rect`], which both share [`BTNS_COLS`]
    /// so draw and hit agree.
    pub min_btn: bool,
    /// Eased progress of this card's assemble animation, `0.0..=1.0`. The
    /// frame draws itself outward from the four corners; `1.0` is a fully
    /// drawn card (and what Motion=off draws immediately).
    pub assemble_t: f32,
    /// Eased progress of the focus-bracket animation, `0.0..=1.0`. The HUD
    /// brackets grow out of the four corners as focus arrives; `1.0` is the
    /// resting focused state (and what Motion=off draws immediately).
    pub focus_t: f32,
    /// Git state of the directory this pane is in, when it is a repo and the
    /// answer has arrived ([`crate::gitfleet`]). Drawn on the top border
    /// after the status glyphs, in whatever room is left before the legend.
    pub git: Option<&'a crate::git::GitInfo>,
    /// Rendered rows worth marking on the gutter — a document's headings, a
    /// review's files and hunks. Drawn under the thumb, so a landmark you are
    /// currently sitting on is not hidden by it.
    pub ticks: &'a [usize],
    /// This pane is pinned to the grid ([`crate::grid`]): marked on the top
    /// border, since a pane that behaves differently from its neighbours has
    /// to say so somewhere.
    pub pinned: bool,
    /// Visible rows where a command started ([`crate::cmdspan`]), ticked on
    /// the left border. Drawn under the error bars: a row can be both, and
    /// "this failed" outranks "this began".
    pub cmd_rows: &'a [u16],
    /// Visible rows holding an error, marked down the LEFT border
    /// ([`crate::errscan`]) — where the failures are, without spending a
    /// column of the program's own grid to say it.
    pub err_rows: &'a [u16],
    /// How long the pane's foreground command has been running, once that is
    /// worth saying ([`crate::runclock`]). `None` for an idle pane.
    pub elapsed: Option<String>,
    /// What the program in this pane says about its own progress (OSC 9;4),
    /// drawn along the bottom border.
    pub progress: Option<crew_term::Progress>,
    /// Rendered rows holding a search hit, drawn over the landmark ticks in
    /// the search's own colour: while you are looking for something, where
    /// the matches are outranks where the sections are.
    pub hits: &'a [usize],
    /// Lines that arrived since this pane was last read, drawn beside the
    /// activity dot ([`crate::unread`]). `0` draws nothing.
    pub unread: usize,
    /// This pane is a document: its gutter is drawn whenever the content is
    /// longer than the pane, not only once it has been scrolled. A shell's
    /// gutter is a scrollback affordance; a document's is where you ARE.
    pub doc: bool,
}

/// Whether `c` is part of a card's frame stroke (as opposed to its legend or a
/// status glyph sitting on the same border).
pub(crate) fn is_frame_glyph(c: char) -> bool {
    matches!(
        c,
        '\u{2500}' | '\u{2502}' | '\u{256d}' | '\u{256e}' | '\u{2570}' | '\u{256f}'
    )
}

/// Hide the border cells a partly-assembled card has not reached yet.
///
/// The frame grows out of its four corners: a cell survives when its distance
/// to the nearest corner, measured along its own edge, is within the reveal
/// length. Corners are therefore drawn first and the middles of the edges last,
/// which is what reads as a frame being *drawn* rather than faded in.
///
/// Only the frame *stroke* is animated. The legend rides the same top border,
/// and clipping it spelled the pane's name out one letter at a time — a card
/// you cannot identify is worse than one that simply appeared. So the reveal
/// tests for box-drawing glyphs and leaves everything else alone: the name is
/// there from the first frame and the frame draws itself around it. Status
/// glyphs and focus brackets are written after this runs and so are untouched
/// either way.
fn assemble(v: &mut Vec<CellView>, cols: u16, rows: u16, t: f32) {
    let t = t.clamp(0.0, 1.0);
    if t >= 1.0 || cols < 2 || rows < 2 {
        return;
    }
    let (hw, hh) = (cols as f32 / 2.0, rows as f32 / 2.0);
    let reach_h = hw * t;
    let reach_v = hh * t;
    v.retain(|c| {
        if !is_frame_glyph(c.c) {
            return true;
        }
        let dx = (c.col as f32 + 0.5 - hw).abs();
        let dy = (c.row as f32 + 0.5 - hh).abs();
        let on_side = c.col == 0 || c.col == cols - 1;
        // Horizontal runs are measured from the nearer left/right corner,
        // vertical runs from the nearer top/bottom one.
        if on_side {
            hh - dy <= reach_v
        } else {
            hw - dx <= reach_h
        }
    });
}

/// Longest a focus bracket grows, in cells down each edge from a corner. Short
/// on purpose: this is a corner mark, not a second border.
const BRACKET_MAX: u16 = 4;

/// Bracket length for a card `rows` tall at eased progress `t`. Bounded to a
/// third of the card so a two-row strip thumbnail can't have its whole side
/// lit up and read as focused-everywhere.
fn bracket_len(rows: u16, t: f32) -> u16 {
    let room = rows.saturating_sub(2) / 3;
    let max = room.min(BRACKET_MAX);
    (max as f32 * t.clamp(0.0, 1.0)).round() as u16
}

/// Recolour the border cells forming the four corner brackets. Vertical only:
/// the top edge carries the legend and the `[-][x]` buttons, and a bracket that
/// overwrote either would be trading information for decoration.
fn draw_brackets(v: &mut [CellView], cols: u16, rows: u16, t: f32) {
    let n = bracket_len(rows, t);
    if n == 0 || cols < 2 || rows < 4 {
        return;
    }
    let accent = crate::palette::accent();
    let (left, right) = (0, cols - 1);
    for i in 0..n {
        for (col, row) in [
            (left, 1 + i),
            (right, 1 + i),
            (left, rows - 2 - i),
            (right, rows - 2 - i),
        ] {
            if let Some(cell) = v.iter_mut().find(|c| c.col == col && c.row == row) {
                cell.fg = accent;
                cell.bold = true;
            }
        }
    }
}

/// Narrowest card (in cells, border included) that carries the border
/// buttons `[-][x]` — below this there's no room for legible click targets,
/// and the pair draws all-or-nothing so hit-tests never half-apply.
const BTNS_COLS: u16 = 13;

/// Pixel rect of one 3-cell border button whose leftmost glyph sits at card
/// column `cols - off`. `None` when the card is too narrow for the pair.
fn btn_rect(rect: Rect, cw: f32, ch: f32, off: u16) -> Option<Rect> {
    let (icols, _) = crate::layout::card_inner_cells(rect.w, rect.h, cw, ch);
    let cols = icols + 2;
    if cols < BTNS_COLS {
        return None;
    }
    Some(Rect {
        x: rect.x + f32::from(cols - off) * cw,
        y: rect.y,
        w: 3.0 * cw,
        h: ch,
    })
}

/// The `[x]` close button: the corner slot (card columns `cols-5 ..= cols-3`).
pub(crate) fn close_btn_rect(rect: Rect, cw: f32, ch: f32) -> Option<Rect> {
    btn_rect(rect, cw, ch, 5)
}

/// The `[-]` minimize button, directly left of `[x]` (columns `cols-8 ..= cols-6`).
pub(crate) fn min_btn_rect(rect: Rect, cw: f32, ch: f32) -> Option<Rect> {
    btn_rect(rect, cw, ch, 8)
}

/// Overwrite (or append) the cell at `(col, row)` in `v` — used to drop status
/// glyphs onto the already-drawn top border.
pub(crate) fn put(
    v: &mut Vec<CellView>,
    col: u16,
    row: u16,
    c: char,
    fg: (u8, u8, u8),
    bold: bool,
) {
    if let Some(cell) = v.iter_mut().find(|x| x.col == col && x.row == row) {
        (cell.c, cell.fg, cell.bg, cell.bold) = (c, fg, crew_theme::theme().page_bg, bold);
    } else {
        v.push(CellView {
            col,
            row,
            c,
            fg,
            bg: crew_theme::theme().page_bg,
            bold,
            italic: false,
            ..Default::default()
        });
    }
}

/// Build the fieldset border for a pane with a `gcols × grows` interior: a
/// rounded card whose top border carries the legend (left) and right-aligned
/// status glyphs. No filled title bar — just the frame on the canvas.
pub(crate) fn pane_card(gcols: u16, grows: u16, b: &Bar) -> Vec<CellView> {
    let (cols, rows) = (gcols + 2, grows + 2);
    // Each pane carries a signature hue derived from its title — the same hash
    // the crew roster uses for agent names, so a swarm agent's pane and its
    // roster row read as one colour. Focus stays legible via bold + the
    // focused border; the unfocused legend recedes toward `legend_off`.
    let hue = crate::chatroster::agent_color(b.title);
    // A card carried over this one lights its whole frame: the drop lands
    // here, and a swap is worth saying before it happens (see `panedrag`).
    let drop_target = crate::panedrag::is_drop_target(b.index.unwrap_or(0) as u16);
    let (border, legend) = if drop_target {
        (crate::palette::accent(), crate::palette::accent())
    } else if b.focused {
        (crew_theme::theme().border_focused, hue)
    } else {
        (
            crew_theme::theme().border_normal,
            crate::anim::lerp_rgb(hue, crew_theme::theme().legend_off, 0.55),
        )
    };
    let label = match b.index {
        Some(n) => format!("{n} {}", b.title),
        None => b.title.to_string(),
    };
    let mut v = titled_card(
        cols,
        rows,
        &label,
        border,
        legend,
        crew_theme::theme().page_bg,
    );
    if v.is_empty() {
        return v;
    }
    // An UNFOCUSED card gets the quiet gradient here, so every frame on the
    // canvas carries the theme's colour and not just the one with focus. The
    // focused card is left flat on purpose: `pane_card_glowing` paints the
    // full drifting ring over it, and that pass only touches cells still
    // wearing `border_focused`.
    if !b.focused {
        crate::modernring::quiet(&mut v, cols, rows, border);
    }
    // Assemble before the status glyphs and brackets are stamped on, so those
    // are never clipped by a half-drawn frame.
    assemble(&mut v, cols, rows, b.assemble_t);
    // The focused card's legend goes bold: the active tile reads at a glance
    // without any extra chrome on the canvas.
    if b.focused {
        for cell in v.iter_mut().filter(|c| c.row == 0 && c.fg == legend) {
            cell.bold = true;
        }
    }
    // Status glyphs ride the top-right border, stepping left from the corner.
    let mut rx = cols.saturating_sub(3);
    // The [-][x] buttons claim the corner slots; status glyphs step past them.
    if b.min_btn && cols >= BTNS_COLS {
        // The pointer lights the button it is on — the one card on the canvas
        // whose legend number matches what `panehover` published this frame.
        let hover = crate::panehover::btn_for(b.index.unwrap_or(0) as u16);
        crate::panebtn::draw(&mut v, cols, legend, hover);
        rx = cols.saturating_sub(10);
    }
    // Both readings of the scroll position share one colour, sampled at the
    // same point of the buffer — see `panescroll::position_fg`.
    let scroll_t =
        crate::panescroll::position(b.total, usize::from(rows.saturating_sub(2)), b.scroll);
    rx = crate::panescroll::count(&mut v, rx, b.scroll, scroll_t);
    // …and the same fact as a shape, down the right border: where you are in
    // the scrollback, not just how far from its bottom. Landmarks first, so
    // the thumb draws over the one you are on.
    crate::panescroll::ticks(&mut v, cols, rows, b);
    crate::panescroll::hit_ticks(&mut v, cols, rows, b);
    crate::panescroll::thumb(&mut v, cols, rows, b);
    crate::panescroll::progress(&mut v, cols, rows, b, crate::anim::now_ms());
    // Where each thing you ran began. Quieter than the error bars, and drawn
    // first so an error on the same row replaces it.
    if !b.cmd_rows.is_empty() {
        let fg = crew_theme::theme().legend_off;
        for &r in b.cmd_rows {
            let row = r + 1;
            if row + 1 < rows {
                put(&mut v, 0, row, '\u{2576}', fg, false);
            }
        }
    }
    // Where the failures are. Content starts one row below the top border,
    // so a content row `n` is border row `n + 1`.
    if !b.err_rows.is_empty() {
        let fg = crew_theme::theme().bell;
        for &r in b.err_rows {
            let row = r + 1;
            if row + 1 < rows {
                put(&mut v, 0, row, '\u{258c}', fg, false);
            }
        }
    }
    // Focus brackets last, so they sit on the finished frame — and only on the
    // focused card, which is the one piece of state they exist to announce.
    if b.focused {
        draw_brackets(&mut v, cols, rows, b.focus_t);
    }
    for (on, c, fg) in [
        (b.broadcast, '»', crew_theme::theme().broadcast),
        (b.activity, '●', crew_theme::theme().activity),
        (b.bell, '!', crew_theme::theme().bell),
    ] {
        if on && rx > 1 {
            put(&mut v, rx, 0, c, fg, false);
            rx = rx.saturating_sub(2);
        }
    }
    // How much arrived while you were reading something else. The dot has
    // always said "something happened here"; the number says how much, which
    // is the difference between glancing over and going back.
    if let Some(n) = crate::unread::badge(b.unread) {
        let w = n.chars().count() as u16;
        if rx > w {
            let start = rx + 1 - w;
            for (i, ch) in n.chars().enumerate() {
                put(
                    &mut v,
                    start + i as u16,
                    0,
                    ch,
                    crew_theme::theme().activity,
                    true,
                );
            }
            rx = start.saturating_sub(2);
        }
    }
    if b.pinned && rx > 1 {
        put(&mut v, rx, 0, '\u{25aa}', crate::palette::accent(), true);
        rx = rx.saturating_sub(2);
    }
    // How long this has been going. Placed before the git badge, since it is
    // the more perishable of the two: a branch does not change while you look
    // away, and a nine-minute build is the thing you came back for.
    if let Some(t) = &b.elapsed {
        let w = t.chars().count() as u16;
        if rx > w + 2 {
            let start = rx + 1 - w;
            for (i, ch) in t.chars().enumerate() {
                put(
                    &mut v,
                    start + i as u16,
                    0,
                    ch,
                    crew_theme::theme().status_fg,
                    false,
                );
            }
            rx = start.saturating_sub(2);
        }
    }
    // The git badge takes what is left of the top border. Its floor is the
    // legend's own last column, read off the cells that were just drawn —
    // the legend is width-clipped by `titled_card`, so asking the drawing is
    // the only way to know where it actually ended.
    if let Some(g) = b.git {
        // Only cells LEFT of the free column count as the legend. The
        // `[-][x]` buttons are drawn in the legend's own colour, at the far
        // right — so scanning the whole row for that colour put the legend's
        // "end" three columns from the corner and left the badge a budget of
        // nothing. It has not drawn on a card with buttons since it landed,
        // which is every full tile.
        let legend_end = v
            .iter()
            .filter(|c| c.row == 0 && c.fg == legend && c.col <= rx)
            .map(|c| c.col)
            .max()
            .unwrap_or(2);
        crate::gitbadge::draw(&mut v, rx, legend_end + 1, g);
    }
    v
}

#[cfg(test)]
#[path = "paneview_tests.rs"]
mod tests;
