//! The pending plan's two buttons: `run` and `discard`, drawn as badges on
//! their own row directly above the composer while a drafted plan waits for
//! its verdict (`ChatPane::plan_pending`).
//!
//! Approving a plan was keyboard-only — Enter on an empty composer, Esc to
//! discard — and the only thing SAYING so was a footer segment. A question
//! addressed to the user deserves a control the pointer can find: each button
//! is a [`crate::segment`] badge (label floored against its block, whatever
//! the theme) that brightens under the pointer and inverts when pressed.
//!
//! The pure half; [`crate::chatplanclick`] owns the click plumbing and send.
use std::ops::Range;

use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatbody::Color;
use crate::glyphs::{fallback, nerd, Glyph};
use crate::segment::{self, Caps, LEFT_CAP, LEFT_CAP_NERD, RIGHT_CAP, RIGHT_CAP_NERD};

/// One of the plan's buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Btn {
    /// Run the plan (`approve`).
    Run,
    /// Throw the plan away (`reject`).
    Discard,
}

/// How long a pressed badge stays inverted, at full motion.
pub(crate) const PRESS_MS: u64 = 120;
/// How far a hovered block moves toward the ink.
const HOVER_LIFT: f32 = 0.2;
/// Column the first badge starts on — the swarm line's left inset.
const INSET: u16 = 1;
/// The key hint after the buttons; the first thing dropped when narrow.
const HINT: &str = "enter \u{00b7} esc";

/// The row's cells, each at its column, and the half-open column span of
/// each button — the hit targets the click path reads.
pub(crate) struct Row {
    pub cells: Vec<(u16, segment::Cell)>,
    pub spans: Vec<(Btn, Range<u16>)>,
}

/// The button's label: its mark, then its verb; the mark alone when `terse`.
fn label(btn: Btn, on: bool, terse: bool) -> String {
    let (glyph, verb) = match btn {
        Btn::Run => (Glyph::Play, "run"),
        Btn::Discard => (Glyph::Fail, "discard"),
    };
    let mark = if on { nerd(glyph) } else { fallback(glyph) };
    if terse {
        mark.to_string()
    } else {
        format!("{mark} {verb}")
    }
}

/// The block colour a button rests on: the accent for `run` (it is the
/// default the footer's `enter` already names), muted for `discard`.
fn block(btn: Btn) -> Color {
    match btn {
        Btn::Run => crate::palette::accent(),
        Btn::Discard => crew_theme::theme().text_muted,
    }
}

/// The badge for `btn` in its current state. Hovered, the block lifts
/// [`HOVER_LIFT`] toward the ink; pressed, block and label swap. `badge`
/// floors the label against whatever block it lands on, so every state
/// reads.
fn badge(btn: Btn, on: bool, terse: bool, hovered: bool, pressed: bool) -> Vec<segment::Cell> {
    let ink = crew_theme::theme().ink;
    let base = block(btn);
    let (fg, bg) = if pressed {
        (base, ink)
    } else if hovered {
        (ink, crate::anim::lerp_rgb(base, ink, HOVER_LIFT))
    } else {
        (segment::page_ink(base), base)
    };
    let mut cells = segment::badge(&label(btn, on, terse), fg, bg, Caps::BOTH);
    // `segment` reads the live icon switch for its caps; this row is drawn
    // for an explicit `on`, so the caps follow the same switch as the mark.
    let (l, r) = [(LEFT_CAP, RIGHT_CAP), (LEFT_CAP_NERD, RIGHT_CAP_NERD)][usize::from(on)];
    if let [first, .., last] = cells.as_mut_slice() {
        first.c = l;
        last.c = r;
    }
    cells
}

/// Lay the row out for `cols` columns, `on` the Nerd Font icon set, with
/// `hover`/`pressed` naming the button in that state. `None` when not even
/// the two terse badges fit — then there is no row to draw.
///
/// Width goes to the buttons first: full labels, then the marks alone; the
/// `enter · esc` hint comes only after both fit with a gap to spare.
pub(crate) fn buttons(
    cols: u16,
    on: bool,
    hover: Option<Btn>,
    pressed: Option<Btn>,
) -> Option<Row> {
    let mut col = INSET;
    let mut row = Row {
        cells: Vec::new(),
        spans: Vec::new(),
    };
    let pair = [Btn::Run, Btn::Discard];
    let width = |terse: bool| -> u16 {
        pair.iter()
            .map(|b| segment::width(&label(*b, on, terse), Caps::BOTH) as u16)
            .sum::<u16>()
            + 1
    };
    let terse = INSET + width(false) > cols;
    if terse && INSET + width(true) > cols {
        return None;
    }
    for (i, btn) in pair.into_iter().enumerate() {
        if i > 0 {
            col += 1;
        }
        let cells = badge(btn, on, terse, hover == Some(btn), pressed == Some(btn));
        let start = col;
        for c in cells {
            row.cells.push((col, c));
            col += crate::chatwidth::char_w(c.c) as u16;
        }
        row.spans.push((btn, start..col));
    }
    let hint_w = crate::chatwidth::str_w(HINT) as u16;
    if col + 2 + hint_w <= cols {
        let theme = crew_theme::theme();
        let muted = crew_theme::readable::enforced(
            theme.text_muted,
            theme.page_bg,
            crew_theme::contrast::text_floor(),
        );
        col += 2;
        for ch in HINT.chars() {
            let cell = segment::Cell {
                c: ch,
                fg: muted,
                bg: None,
                bold: false,
            };
            row.cells.push((col, cell));
            col += crate::chatwidth::char_w(ch) as u16;
        }
    }
    Some(row)
}

/// Rows the button row claims: 1 while a plan is pending and the pane is
/// wide enough for the two badges, else 0. Clock-independent, as every row
/// budget must be (`chatplace::grants`).
pub(crate) fn plan_rows(pane: &ChatPane, cols: u16) -> u16 {
    let on = crate::glyphs::on();
    u16::from(pane.plan_pending && buttons(cols, on, None, None).is_some())
}

/// Whether the pressed badge is still showing its invert at `now`.
pub(crate) fn pressed_btn(pane: &ChatPane, now: u64) -> Option<Btn> {
    pane.press_btn
        .filter(|(_, flash)| flash.live(now))
        .map(|(b, _)| b)
}

/// Draw the row at `row` for the pane's current hover and press state, on
/// the animation clock (the press flash is what it reads).
pub(crate) fn row_cells(pane: &ChatPane, cols: u16, row: u16) -> Vec<CellView> {
    let on = crate::glyphs::on();
    let pressed = pressed_btn(pane, crate::anim::now_ms());
    let Some(laid) = buttons(cols, on, pane.hover_btn, pressed) else {
        return Vec::new();
    };
    let page = crew_theme::theme().page_bg;
    laid.cells
        .into_iter()
        .filter(|(col, _)| *col < cols)
        .map(|(col, c)| CellView {
            col,
            row,
            c: c.c,
            fg: c.fg,
            bg: c.bg.unwrap_or(page),
            bold: c.bold,
            italic: false,
            ..Default::default()
        })
        .collect()
}

#[cfg(test)]
#[path = "chatplanbtn_tests.rs"]
mod tests;
