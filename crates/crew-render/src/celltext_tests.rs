use super::*;

fn params(family: Option<String>) -> FontParams {
    FontParams {
        font_size: 14.0,
        line_height: 17.5,
        cell_w: 14.0 * 0.6,
        family,
    }
}

#[test]
fn cell_metrics_larger_font_gives_larger_dimensions() {
    let small = cell_metrics(12.0);
    let large = cell_metrics(24.0);
    assert!(large.0 > small.0, "cell_w should grow with font size");
    assert!(large.1 > small.1, "cell_h should grow with font size");
    assert_eq!(large.1, 24.0 * 1.25, "cell_h is 1.25× font size");
}

#[test]
fn cell_metrics_height_is_125_percent() {
    assert_eq!(cell_metrics(16.0).1, 20.0);
}

#[test]
fn cell_metrics_are_family_independent_and_whole_pixel() {
    // The whole point of the fixed box: the same size gives the same cell no
    // matter which family the user picks — snapped to whole physical pixels
    // (14 × 0.6 = 8.4 → 8, 14 × 1.25 = 17.5 → 18) so glyphs never smear.
    assert_eq!(cell_metrics(14.0), (8.0, 18.0));
    let (w, h) = cell_metrics(28.0); // 2x display
    assert_eq!((w.fract(), h.fract()), (0.0, 0.0));
}

#[test]
fn family_from_maps_named_and_default() {
    match family_from(&Some("Menlo".to_string())) {
        Family::Name(n) => assert_eq!(n, "Menlo"),
        _ => panic!("named family should map to Family::Name"),
    }
    assert!(matches!(family_from(&None), Family::Monospace));
    assert!(matches!(
        family_from(&Some(String::new())),
        Family::Monospace
    ));
}

#[test]
fn bold_glyphs_snap_to_the_same_cell_advance() {
    // The fixed cell box must hold for BOLD runs too — a bold face's natural
    // advances differ from the regular face's, so if `set_monospace_width`
    // ever stopped covering weight variants, bold text would drift off-grid.
    let style = |col: u16, c: char, bold: bool| CellView {
        col,
        row: 0,
        c,
        fg: (200, 200, 200),
        bg: (0, 0, 0),
        bold,
        italic: false,
    };
    let mut fs = FontSystem::new();
    let cells = vec![
        style(0, 'W', true),
        style(1, 'i', true),
        style(2, 'm', false),
        style(3, '0', true),
    ];
    let (cell_w, cell_h) = cell_metrics(14.0);
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
    };
    let buf = build_pane_buffer(&mut fs, &cells, 4, 1, 4.0 * cell_w, cell_h, &p);
    let runs: Vec<_> = buf.layout_runs().collect();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].glyphs.len(), 4, "four columns shape to four glyphs");
    for g in runs[0].glyphs {
        let cols = g.x / cell_w;
        assert!(
            (cols - cols.round()).abs() < 1e-3,
            "glyph at x={} is off the {cell_w}px grid",
            g.x
        );
    }
}

#[test]
fn build_pane_buffer_lays_out_grid_with_styles() {
    let mut fs = FontSystem::new();
    let cells = vec![
        CellView {
            col: 0,
            row: 0,
            c: 'h',
            fg: (200, 200, 200),
            bg: (0, 0, 0),
            bold: true,
            italic: false,
        },
        CellView {
            col: 1,
            row: 0,
            c: 'i',
            fg: (10, 20, 30),
            bg: (0, 0, 0),
            bold: false,
            italic: true,
        },
        // row 1 left empty at col 0 → exercises the None-gap branch
        CellView {
            col: 1,
            row: 1,
            c: 'x',
            fg: (1, 2, 3),
            bg: (0, 0, 0),
            bold: false,
            italic: false,
        },
    ];
    let buf = build_pane_buffer(&mut fs, &cells, 3, 2, 24.0, 36.0, &params(None));
    assert!(
        buf.layout_runs().count() >= 1,
        "buffer should lay out lines"
    );
}

#[test]
fn build_pane_buffer_handles_empty_cells() {
    let mut fs = FontSystem::new();
    // Empty family string also exercises the system-monospace fallback.
    let buf = build_pane_buffer(&mut fs, &[], 2, 2, 16.0, 32.0, &params(Some(String::new())));
    assert!(buf.layout_runs().count() <= 2);
}

#[test]
fn adjacent_same_style_cells_coalesce_into_one_span() {
    // Three same-styled cells on row 0 should collapse to a single shaping run.
    let style = |col: u16, c: char| CellView {
        col,
        row: 0,
        c,
        fg: (200, 200, 200),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
    };
    let mut fs = FontSystem::new();
    let cells = vec![style(0, 'a'), style(1, 'b'), style(2, 'c')];
    let buf = build_pane_buffer(&mut fs, &cells, 3, 1, 16.0, 20.0, &params(None));
    // One physical line, and the glyphs spell "abc" in order.
    let runs: Vec<_> = buf.layout_runs().collect();
    assert_eq!(runs.len(), 1, "single row lays out one line");
    let glyphs = runs[0].glyphs.len();
    assert_eq!(glyphs, 3, "three columns shape to three glyphs");
}

#[test]
fn build_pane_buffer_ignores_out_of_range_cells() {
    let mut fs = FontSystem::new();
    // A cell beyond cols/rows must be dropped without panicking.
    let cells = vec![CellView {
        col: 9,
        row: 9,
        c: 'z',
        fg: (1, 1, 1),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
    }];
    let _ = build_pane_buffer(&mut fs, &cells, 2, 2, 16.0, 32.0, &params(None));
}
