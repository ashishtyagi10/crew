use super::*;
use crate::scenecache::pane_sig;
use glyphon::FontSystem;

fn params() -> FontParams {
    FontParams {
        font_size: 14.0,
        line_height: 17.5,
        cell_w: 14.0 * 0.6,
        family: None,
    }
}

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
fn pane_sig_is_stable_and_content_sensitive() {
    let p1 = pane(vec![cell(0, 0, 'a', (1, 2, 3))], false, false);
    let p2 = pane(vec![cell(0, 0, 'a', (1, 2, 3))], false, false);
    let p3 = pane(vec![cell(0, 0, 'b', (1, 2, 3))], false, false);
    assert_eq!(
        pane_sig(&p1, 10, 2, &params()),
        pane_sig(&p2, 10, 2, &params())
    );
    assert_ne!(
        pane_sig(&p1, 10, 2, &params()),
        pane_sig(&p3, 10, 2, &params())
    );
    assert_ne!(
        pane_sig(&p1, 10, 2, &params()),
        pane_sig(&p1, 12, 2, &params()),
        "grid dims are part of the signature"
    );
}

#[test]
fn pane_sig_ignores_position_but_not_size() {
    let mut moved = pane(vec![cell(0, 0, 'a', (1, 2, 3))], false, false);
    let sig_before = pane_sig(&moved, 10, 2, &params());
    moved.x = 500.0;
    moved.y = 300.0;
    assert_eq!(sig_before, pane_sig(&moved, 10, 2, &params()));
    moved.w = 160.0;
    assert_ne!(sig_before, pane_sig(&moved, 10, 2, &params()));
}

#[test]
fn unchanged_pane_reuses_last_frames_buffer() {
    let mut fs = FontSystem::new();
    let panes = vec![pane(vec![cell(0, 0, 'a', (1, 2, 3))], false, false)];
    let (_q, bufs, sigs, _bd) = build_scene(
        &panes,
        8.0,
        16.0,
        &mut fs,
        &params(),
        false,
        false,
        (vec![], vec![]),
    );
    // Second frame, same content: the same signature comes back out and the
    // build succeeds while consuming the previous pass (buffer moved through).
    let (_q2, bufs2, sigs2, _bd2) = build_scene(
        &panes,
        8.0,
        16.0,
        &mut fs,
        &params(),
        false,
        false,
        (sigs.clone(), bufs),
    );
    assert_eq!(sigs, sigs2);
    assert_eq!(bufs2.len(), 1);
}
