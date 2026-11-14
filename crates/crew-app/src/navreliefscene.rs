//! The scenes of the open nav's relief tiles ([`crate::navrelief`]): one
//! bare sheet per tile, slipped in under the nav's words, and the slab
//! that used to lie under the whole column taken away.
use crew_render::{CellView, PaneScene};

use crate::layout::Rect;
use crate::navrelief::{rect_of, tiles, unruled};

/// The sheet of one tile: a card scene with nothing written on it. Its two
/// blank corner cells are what let it stretch to `r` exactly instead of to
/// whole cells (`crew_render`'s stretch split).
fn sheet(r: Rect, cw: f32, ch: f32, lifted: bool) -> PaneScene {
    let page = crew_theme::theme().page_bg;
    let blank = |col: u16, row: u16| CellView {
        col,
        row,
        c: ' ',
        bg: page,
        ..Default::default()
    };
    let lc = ((r.w / cw).floor() as u16).saturating_sub(1).max(1);
    let lr = ((r.h / ch).floor() as u16).saturating_sub(1).max(1);
    PaneScene {
        cells: vec![blank(0, 0), blank(lc, lr)],
        x: r.x,
        y: r.y,
        w: r.w,
        h: r.h,
        focused: false,
        bordered: false,
        glass: true,
        scan: -1.0,
        lift: if lifted { 1.0 } else { 0.0 },
        glint: -1.0,
        pictures: Vec::new(),
        stretch: true,
        overlay: false,
        paint: Vec::new(),
    }
}

/// Turn the nav card just pushed onto `scenes` (its interior, then its
/// frame: [`crate::panelcard::push_framed`]) into tiles. A no-op off the
/// relief themes, and on a nav with no section to raise.
pub(crate) fn tile(scenes: &mut Vec<PaneScene>, rect: Rect, cw: f32, ch: f32, gap: f32) {
    if crew_theme::theme().relief() <= 0.0 || scenes.len() < 2 {
        return;
    }
    let at = scenes.len() - 2;
    let tiles = tiles(&scenes[at].cells);
    if tiles.is_empty() {
        return;
    }
    unruled(&mut scenes[at].cells, &tiles);
    // The slab under the whole column goes: its sheet, and the frame's
    // rules with it (the legend stays, a heading on the page).
    let frame = &mut scenes[at + 1];
    frame.glass = false;
    for c in frame.cells.iter_mut() {
        if ('\u{2500}'..='\u{257f}').contains(&c.c) {
            c.c = ' ';
        }
    }
    // Under the words: the sheets go in ahead of the interior.
    let sheets: Vec<PaneScene> = tiles
        .iter()
        .map(|t| sheet(rect_of(t, rect, ch, gap), cw, ch, t.lifted))
        .collect();
    scenes.splice(at..at, sheets);
}
