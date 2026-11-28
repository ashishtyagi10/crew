use crate::celltext::{build_pane_buffer, cell_metrics, FontParams, CELL_H_RATIO};
use crate::CellView;
use crew_theme::deco::{CursorMark, CursorShape};

/// `(glyph id, x)` of every glyph of `a=>b` shaped in the embedded face.
fn shape(ligatures: bool, cursor_at: Option<u16>) -> Vec<(u16, f32)> {
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let cells: Vec<CellView> = "a=>b"
        .chars()
        .enumerate()
        .map(|(i, c)| CellView {
            col: i as u16,
            c,
            fg: (200, 200, 200),
            cursor: if cursor_at == Some(i as u16) {
                CursorMark {
                    shape: CursorShape::Beam,
                    color: (255, 255, 255),
                }
            } else {
                CursorMark::default()
            },
            ..Default::default()
        })
        .collect();
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 400,
        smooth: 0,
        gamma: 0,
        ligatures,
        dark: true,
        body: ((255, 255, 255), (0, 0, 0)),
    };
    let mut fs = crate::embedfont::font_system();
    let buf = build_pane_buffer(&mut fs, &cells, 4, 1, 4.0 * cell_w, cell_h, &p);
    let runs: Vec<_> = buf.layout_runs().collect();
    runs[0].glyphs.iter().map(|g| (g.glyph_id, g.x)).collect()
}

#[test]
fn the_face_joins_an_arrow_and_the_grid_holds() {
    let (on, off) = (shape(true, None), shape(false, None));
    assert_eq!(on.len(), 4, "a ligature is still one glyph per cell");
    assert_ne!(on[1].0, off[1].0, "`=` of `=>` is the ligature's half");
    // The cell after the ligature starts exactly where it does without it.
    assert_eq!(on[3], off[3]);
    assert_eq!(on[0], off[0]);
}

#[test]
fn the_setting_changes_the_attrs_and_the_caret_does_not() {
    use crate::ligature::shape;
    use glyphon::Attrs;
    let base = Attrs::new();
    assert_eq!(shape(base.clone(), true), base);
    let off = shape(base.clone(), false);
    assert!(!off.font_features.features.is_empty());
    assert!(off.compatible(&base), "features never split a run");
}

#[test]
fn the_caret_leaves_a_ligature_joined() {
    // No split: a stretch break picks a narrower face in families that have
    // one, and a blinking caret would join and split it every blink.
    assert_eq!(shape(true, Some(2)), shape(true, None));
}
