//! Cards in RELIEF: neumorphism (crew-theme `LiquidStyle::relief`; the user,
//! 2026-11-12: "no border needed on pane, just an elevated card").
//!
//! On these themes a card is the page's own colour and is told from the page
//! by two soft shadows, which the glass pass draws (`neu` in `glass.wgsl`).
//! This is the rest of it: the frame is not stroked, the corners round
//! further, and the card reaches its cells' outer edges, legends and all. The
//! shadows keep the page's own light and dark on every card (the user,
//! 2026-11-12: "shadow has wrong color", of a focused card's accent glow):
//! the card with the keys is told by its height alone.
use crate::scene::PaneScene;

/// Whether `pane` is a card drawn in relief: a card scene (it carries the
/// sheet) on a relief theme. A pop-up is one too: an unframed box of the
/// page's colour standing higher than every pane ([`POPUP_LIFT`]), its two
/// shadows saying where it ends.
pub(crate) fn on(pane: &PaneScene) -> bool {
    pane.glass && crew_theme::theme().relief() > 0.0
}

/// How high a pop-up stands in relief: the shader's top step (`NEU_TOP`).
pub(crate) const POPUP_LIFT: f32 = 1.5;

/// A rule of the frame: the box-drawing block, light and heavy.
fn is_rule(c: char) -> bool {
    ('\u{2500}'..='\u{257F}').contains(&c)
}

/// `pane` as drawn in relief: every rule on its outer rows and columns
/// blanked. The legends, the status glyphs and the marks that stand on those
/// rows stay, and so does any rule inside the card (a divider, a table).
pub(crate) fn unframed(pane: &PaneScene) -> PaneScene {
    let last_col = pane.cells.iter().map(|c| c.col).max().unwrap_or(0);
    let last_row = pane.cells.iter().map(|c| c.row).max().unwrap_or(0);
    let mut out = pane.clone();
    for c in out.cells.iter_mut().filter(|c| is_rule(c.c)) {
        if c.row == 0 || c.row == last_row || c.col == 0 || c.col == last_col {
            c.c = ' ';
        }
    }
    out
}

/// A relief card's corner radius: most of a cell, far rounder than a stroked
/// frame's arc. The content starts a full cell in from the frame, so a
/// corner this round still clears it. Square corners were tried for a day
/// (2026-11-13) and taken back (the user, 2026-11-14: "I didn't like 'no
/// rounded' corner look ... go back to rounded corner"); the tighter gutter
/// from that day stays.
pub(crate) fn radius(cell_h: f32) -> f32 {
    (cell_h * 0.9).round()
}

/// The least gutter two relief cards keep between them, in logical px: room
/// for the near part of both shadows, which is what tells the cards apart.
/// The density's own gap stands when it is wider; nothing is added on top
/// of it (the user, 2026-11-13: "padding between panels should be reduced").
pub const SHADOW_ROOM: f32 = 6.0;

#[cfg(test)]
#[path = "relief_tests.rs"]
mod tests;
