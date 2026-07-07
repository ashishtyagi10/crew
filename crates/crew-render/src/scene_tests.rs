use super::*;
use crate::cellgrid::{default_bg, CellView};
use crate::celltext::FontParams;
use glyphon::FontSystem;

fn cell(col: u16, row: u16, c: char, bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg: (200, 200, 200),
        bg,
        bold: false,
        italic: false,
    }
}

fn params() -> FontParams {
    FontParams {
        font_size: 14.0,
        line_height: 17.5,
        cell_w: 14.0 * 0.6,
        family: None,
        weight: 400,
    }
}

fn pane(cells: Vec<CellView>, bordered: bool, overlay: bool) -> PaneScene {
    PaneScene {
        cells,
        x: 0.0,
        y: 0.0,
        w: 80.0,
        h: 40.0,
        focused: false,
        bordered,
        overlay,
    }
}

#[test]
fn bg_quads_only_for_non_default_cells() {
    let mut fs = FontSystem::new();
    let panes = vec![pane(
        vec![cell(0, 0, 'a', default_bg()), cell(1, 0, 'b', (10, 20, 30))],
        false,
        false,
    )];
    let (quads, buffers, _sigs, borders) = build_scene(
        &panes,
        8.0,
        16.0,
        &mut fs,
        &params(),
        false,
        false,
        (vec![], vec![]),
    );
    assert_eq!(quads.len(), 1, "only the non-default-bg cell gets a quad");
    assert_eq!(buffers.len(), 1);
    assert!(borders.is_empty());
    assert_eq!(quads[0].x, 8.0); // positioned at col 1
    assert_eq!(quads[0].color[3], 1.0); // opaque
}

#[test]
fn bordered_pane_emits_a_border() {
    let mut fs = FontSystem::new();
    let (_q, _b, _s, borders) = build_scene(
        &[pane(vec![], true, false)],
        8.0,
        16.0,
        &mut fs,
        &params(),
        false,
        false,
        (vec![], vec![]),
    );
    assert_eq!(borders.len(), 1);
}

#[test]
fn want_overlay_partitions_panes() {
    let mut fs = FontSystem::new();
    let panes = vec![
        pane(vec![cell(0, 0, 'x', (1, 2, 3))], true, false),
        pane(vec![cell(0, 0, 'y', (4, 5, 6))], false, true),
    ];
    // Base pass: only the non-overlay pane (bordered → one border).
    let (q, b, _s, bd) = build_scene(
        &panes,
        8.0,
        16.0,
        &mut fs,
        &params(),
        false,
        false,
        (vec![], vec![]),
    );
    assert_eq!((q.len(), b.len(), bd.len()), (1, 1, 1));
    // Overlay pass: only the overlay pane (bordered:false → no border). Two
    // quads: the full-rect black backdrop plus the one non-default-bg cell.
    let (q2, b2, _s2, bd2) = build_scene(
        &panes,
        8.0,
        16.0,
        &mut fs,
        &params(),
        true,
        false,
        (vec![], vec![]),
    );
    assert_eq!((q2.len(), b2.len(), bd2.len()), (2, 1, 0));
}

#[test]
fn overlay_pane_gets_an_opaque_page_bg_backdrop() {
    let mut fs = FontSystem::new();
    // An overlay pane with only default-bg cells still gets a backdrop.
    let panes = vec![pane(vec![cell(0, 0, 'y', default_bg())], false, true)];
    let (quads, _b, _s, _bd) = build_scene(
        &panes,
        8.0,
        16.0,
        &mut fs,
        &params(),
        true,
        false,
        (vec![], vec![]),
    );
    assert_eq!(quads.len(), 1, "the backdrop quad, no per-cell quad");
    let q = &quads[0];
    assert_eq!((q.x, q.y, q.w, q.h), (0.0, 0.0, 80.0, 40.0)); // spans the pane
    let t = crew_theme::theme();
    let (r, g, b) = t.page_bg;
    assert_eq!(
        q.color,
        [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
    );
}

#[test]
fn focused_border_is_brighter_than_unfocused() {
    let mut fs = FontSystem::new();
    let mut p = pane(vec![], true, false);
    p.focused = true;
    let (_q, _b, _s, focused) = build_scene(
        &[p],
        8.0,
        16.0,
        &mut fs,
        &params(),
        false,
        false,
        (vec![], vec![]),
    );
    let (_q2, _b2, _s2, normal) = build_scene(
        &[pane(vec![], true, false)],
        8.0,
        16.0,
        &mut fs,
        &params(),
        false,
        false,
        (vec![], vec![]),
    );
    let t = crew_theme::theme();
    let f = |c: (u8, u8, u8)| {
        [
            c.0 as f32 / 255.0,
            c.1 as f32 / 255.0,
            c.2 as f32 / 255.0,
            1.0,
        ]
    };
    assert_eq!(focused[0].color, f(t.border_focused));
    assert_eq!(normal[0].color, f(t.border_normal));
}
