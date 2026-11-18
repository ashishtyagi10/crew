//! The scenes of the open nav's relief panel ([`crate::navrelief`]), and
//! the bare sheet every relief well and tab card is drawn as.
use crew_render::{CellView, PaneScene};

use crate::layout::Rect;

/// The sheet of a well (`lift` below zero: [`crate::reliefwell`]): a card
/// scene with nothing written on it. Its two
/// blank corner cells are what let it stretch to `r` exactly instead of to
/// whole cells (`crew_render`'s stretch split).
pub(crate) fn sheet(r: Rect, cw: f32, ch: f32, lift: f32) -> PaneScene {
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
        lift,
        glint: -1.0,
        pictures: Vec::new(),
        stretch: true,
        overlay: false,
        paint: Vec::new(),
    }
}

/// Mould the nav card just pushed onto `scenes` (its interior, then its
/// frame: [`crate::panelcard::push_framed`]) as one relief panel: the slab
/// under the whole column stays (the renderer strokes no frame on it), a
/// groove parts each section from the one above, and every tab's card is
/// sunk into the slab as a well. A no-op off the relief themes.
pub(crate) fn panel(scenes: &mut Vec<PaneScene>, cw: f32, ch: f32, gap: f32) {
    if crew_theme::theme().relief() <= 0.0 || scenes.len() < 2 {
        return;
    }
    let at = scenes.len() - 2;
    let nav = &mut scenes[at];
    let cols = (nav.w / cw.max(1.0)).round() as u16;
    let ys = crate::navrelief::grooves(&nav.cells);
    crate::navrelief::unruled(&mut nav.cells);
    nav.paint
        .extend(crate::navrelief::groove_paint(&ys, cols, ch));
    let tabs = crate::reliefwell::boxes(&nav.cells);
    let on = crate::navreliefactive::active(&nav.cells, &tabs, crate::palette::accent());
    crate::reliefwell::unstroke(&mut nav.cells, &tabs);
    let origin = (nav.x, nav.y);
    // After the frame: its slab is drawn first, the wells are sunk into it.
    scenes.extend(tabs.iter().map(|b| {
        let r = crate::navrelief::well_rect(b, origin, cw, ch, gap);
        sheet(r, cw, ch, crate::navreliefactive::depth(on == Some(*b)))
    }));
}
