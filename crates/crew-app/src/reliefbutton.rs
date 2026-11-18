//! A button on a soft card is a raised key, not a slab of fill.
//!
//! On the relief palettes (soft's four, metal's two) every box a pane draws
//! is pressed in as a well ([`crate::reliefwell`]), the cards and the input
//! bar are moulded — and the settings form's Save and Cancel were still two
//! flat bars of fill, square-shouldered and one cell tall, the one thing on
//! the card that looked printed on it (the open list of 2026-11-14: "flat
//! Save/Cancel and chips on soft"). A neumorphic control stands proud of
//! the surface it sits on, the way the cards stand proud of the page. So a
//! raised sheet goes under each button: its fill and label ride on it as
//! they did, and its edge and shadow say "press me". Read off the cells,
//! like the wells: a button is a run of one fill, bordered by the page,
//! padded at both ends with the no-break spaces `settingspane::widgets`
//! gives a button (a plain space with a fill draws a `█`).
use crew_render::CellView;

use crate::layout::Rect;

/// How far a button stands off its card: a third of a focused card's lift
/// (1.0), so a key never out-shouts the card it is on.
pub(crate) const RAISE: f32 = 0.35;
/// How far the plate stands out past the label at each end, in cells.
const OUT: f32 = 0.1;
/// How far the plate stands above and below its row, in cells: a key a
/// little taller than its line, its label centred on it.
const RISE: f32 = 0.2;

/// One button: row, first and last column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Key {
    pub row: u16,
    pub c0: u16,
    pub c1: u16,
}

/// Every button drawn in `cells` over `page`.
pub(crate) fn keys(cells: &[CellView], page: (u8, u8, u8)) -> Vec<Key> {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.bg != page).collect();
    v.sort_unstable_by_key(|c| (c.row, c.col));
    let mut out = Vec::new();
    let mut i = 0;
    while i < v.len() {
        let mut j = i;
        while j + 1 < v.len()
            && v[j + 1].row == v[i].row
            && v[j + 1].col == v[j].col + 1
            && v[j + 1].bg == v[i].bg
        {
            j += 1;
        }
        let pad = |c: &CellView| c.c == '\u{a0}';
        if j >= i + 3 && pad(v[i]) && pad(v[j]) && !v[i].mark {
            out.push(Key {
                row: v[i].row,
                c0: v[i].col,
                c1: v[j].col,
            });
        }
        i = j + 1;
    }
    out
}

/// Where `k`'s plate lies, for a scene at `(x, y)` with `cw`×`ch` cells.
pub(crate) fn rect_of(k: &Key, (x, y): (f32, f32), cw: f32, ch: f32) -> Rect {
    Rect {
        x: x + (k.c0 as f32 - OUT) * cw,
        y: y + (k.row as f32 - RISE) * ch,
        w: ((k.c1 - k.c0 + 1) as f32 + 2.0 * OUT) * cw,
        h: (1.0 + 2.0 * RISE) * ch,
    }
}

/// Lift each key's label off its fill onto the plate: the fill goes (the
/// plate is the key now), and a label drawn on a coloured fill — the
/// primary one — takes that colour as its ink, floored on the card, so
/// Save still reads as the one to press.
pub(crate) fn onto_plate(cells: &mut [CellView], keys: &[Key], page: (u8, u8, u8)) {
    let floor = crew_theme::contrast::text_floor();
    for k in keys {
        let on = |c: &CellView| c.row == k.row && (k.c0..=k.c1).contains(&c.col);
        let Some(fill) = cells.iter().find(|c| on(c)).map(|c| c.bg) else {
            continue;
        };
        let hued = crew_theme::oklch::from_srgb(fill).c > HUED;
        let t = crew_theme::theme();
        for c in cells.iter_mut().filter(|c| on(c)) {
            let ink = if hued { fill } else { c.fg };
            let floored = crew_theme::readable::enforced(ink, page, floor);
            // A dark plate: the accent at Lc 60, level with Cancel's ink.
            c.fg = crew_theme::darkread::lift(t, ink, floored, page);
            c.bg = page;
        }
    }
}

/// OKLCH chroma past which a fill is a colour (the accent), not a grey.
const HUED: f32 = 0.06;

#[cfg(test)]
#[path = "reliefbutton_tests.rs"]
mod tests;
