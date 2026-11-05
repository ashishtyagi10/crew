use glyphon::SwashCache;

use super::*;
use crate::cellgrid::CellView;
use crate::celltext::{build_pane_buffer, cell_metrics, FontParams, CELL_H_RATIO};

/// At 26 px the fallback face's taller line drops the `▣` row's baseline
/// from 25.57 to 25.49 below its top: one pixel once rounded, which is the
/// broken stroke the inbox's "pictures hidden" row showed. The `▣` row
/// comes first so the stroke it caches is the one every row reuses.
const ROWS: [&str; 3] = [
    "\u{2502} \u{25a3} 1 \u{2502}",
    "\u{2502} abc \u{2502}",
    "\u{2502} ab  \u{2502}",
];

fn grid() -> (glyphon::FontSystem, Buffer, f32) {
    let mut cells = Vec::new();
    for (row, text) in ROWS.iter().enumerate() {
        for (col, c) in text.chars().enumerate() {
            cells.push(CellView {
                col: col as u16,
                row: row as u16,
                c,
                fg: (200, 200, 200),
                bg: (0, 0, 0),
                ..Default::default()
            });
        }
    }
    let mut fs = crate::embedfont::font_system();
    let (cell_w, cell_h) = cell_metrics(26.0, CELL_H_RATIO);
    let p = FontParams {
        font_size: 26.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 500,
        smooth: 0,
        gamma: 0,
        dark: true,
        body: ((255, 255, 255), (0, 0, 0)),
    };
    let buf = build_pane_buffer(&mut fs, &cells, 7, 3, 7.0 * cell_w, 3.0 * cell_h, &p);
    (fs, buf, cell_h)
}

#[test]
fn a_row_with_a_fallback_glyph_is_drawn_on_the_grids_baseline() {
    let (_fs, buf, _) = grid();
    let own: Vec<f32> = buf.layout_runs().map(|r| r.line_y - r.line_top).collect();
    assert_ne!(
        own[0].round(),
        own[1].round(),
        "the fixture no longer has a fallback row off the grid's baseline: {own:?}"
    );
    assert_eq!(
        baseline(&buf),
        Some(own[1]),
        "the plain rows' baseline is the grid's"
    );
}

#[test]
fn a_card_stroke_covers_its_cell_on_every_row() {
    let (mut fs, buf, cell_h) = grid();
    let mut swash = SwashCache::new();
    let (w, h) = (buf.size().0.unwrap_or(0.0), buf.size().1.unwrap_or(0.0));
    let panes = vec![(buf, 0.0, 0.0, w, h, Vec::new())];
    crate::smoothing::presmooth(&mut swash, &mut fs, &panes);
    let buf = &panes[0].0;
    let base = baseline(buf);
    for run in buf.layout_runs() {
        let stroke = run.glyphs.first().expect("a stroke opens the row");
        let phys = stroke.physical((0.0, 0.0), 1.0);
        let img = swash.image_cache[&phys.cache_key].as_ref().expect("drawn");
        // Where glyphon puts it: the line's rounded baseline less the
        // image's own placement (vendored text_render.rs).
        let top = line_y(&run, base).round() as i32 + phys.y - img.placement.top;
        assert_eq!(
            (top, top + img.placement.height as i32),
            (run.line_top as i32, (run.line_top + cell_h) as i32),
            "row {}'s stroke left its cell",
            run.line_i
        );
    }
}

/// Patch 6 of the vendored glyphon: without it the baseline crew picks
/// never reaches the draw, and the strokes cut for it land a pixel off.
#[test]
fn glyphon_draws_on_the_baseline_crew_gives_it() {
    let lib = include_str!("../../../vendor/glyphon/src/lib.rs");
    let render = include_str!("../../../vendor/glyphon/src/text_render.rs");
    assert!(
        lib.contains("pub baseline: Option<f32>,"),
        "TextArea lost `baseline`"
    );
    assert!(
        render.contains(".map_or(run.line_y, |b| run.line_top + b)"),
        "text_render.rs no longer draws on the area's baseline"
    );
}
