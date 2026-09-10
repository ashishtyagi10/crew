//! Where a composer pop-up sits, and how wide it is. The "commands" palette,
//! the attach picker, the model picker, the key prompt, Cmd+F and Ctrl+R all
//! stand ABOVE the focused crew pane's composer, flush with its left edge —
//! a menu growing out of the prompt, the shape every dropdown has taught.
//!
//! The composer is not the pane's last rows: `chatplace::grants` puts the
//! summary footer (model · cost · mode line, up to three rows) under it.
//! Every pop-up used to subtract the composer alone, so it landed on the
//! composer itself and hid the very text being chosen. And every pop-up
//! used to be as wide as the pane, its list centred in a band of nothing:
//! on a full-window pane, a five-row picker was a box the width of the
//! screen with forty characters in the middle. One function, one source of
//! the composer's top (the same grants the pane is drawn from); one width
//! rule, the card's own measure.
use crate::chat::ChatPane;
use crate::layout::Rect;
use crew_render::{CellView, PaneScene};

/// A composer pop-up as its builder hands it over: the card's cells, laid
/// out in `cols` × `rows`. `scene` turns it into the overlay on the pane.
pub(crate) struct Popup {
    pub cells: Vec<CellView>,
    pub cols: u16,
    pub rows: u16,
}

/// A card is never narrower than this: a one-word list is still a card,
/// with room for its legend and the hint on its bottom edge.
pub(crate) const MIN_COLS: u16 = 36;

/// The card's width in cells for `content` columns of rows inside a pane
/// `cols` wide: the content plus its two border columns, floored at
/// [`MIN_COLS`], never wider than the pane.
pub(crate) fn card_cols(content: usize, cols: u16) -> u16 {
    let want = u16::try_from(content.saturating_add(2)).unwrap_or(u16::MAX);
    want.max(MIN_COLS).min(cols)
}

/// The y (surface px) of a pop-up `mh` px tall whose bottom edge meets the
/// composer's top edge in pane `r`; clamped at the pane's top when the
/// pop-up is taller than the room above.
pub(crate) fn above_composer(pane: &ChatPane, r: Rect, cw: f32, ch: f32, mh: f32) -> f32 {
    let cols = (r.w / cw).floor() as u16;
    let rows = (r.h / ch).floor() as u16;
    let g = crate::chatplace::grants(pane, cols, rows);
    let composer_top = r.y + f32::from(rows.saturating_sub(g.bottom)) * ch;
    (composer_top - mh).max(r.y)
}

/// The pop-up as an overlay scene on pane `r`: standing on the composer,
/// flush with the pane's left edge (the composer's own left border), as
/// wide as its cells. Overlay, so the overlay pass backs it with an opaque
/// page and a sheer window holds it solid.
pub(crate) fn scene(pane: &ChatPane, r: Rect, cw: f32, ch: f32, p: Popup) -> PaneScene {
    let h = f32::from(p.rows) * ch;
    PaneScene {
        cells: p.cells,
        x: r.x,
        y: above_composer(pane, r, cw, ch, h),
        w: f32::from(p.cols) * cw,
        h,
        focused: false,
        bordered: false,
        glass: false,
        scan: -1.0,
        overlay: true,
        paint: Vec::new(),
    }
}

#[cfg(test)]
#[path = "popupplace_tests.rs"]
mod tests;
