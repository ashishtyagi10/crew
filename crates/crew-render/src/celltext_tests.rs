use super::*;

fn params(family: Option<String>) -> FontParams {
    FontParams {
        font_size: 14.0,
        line_height: 17.5,
        cell_w: 14.0 * 0.6,
        family,
        weight: 400,
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
        weight: 400,
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
fn medium_weight_glyphs_snap_to_the_same_cell_advance() {
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
    let cells = vec![style(0, 'W'), style(1, 'i'), style(2, 'm'), style(3, '0')];
    let (cell_w, cell_h) = cell_metrics(14.0);
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 500,
    };
    let buf = build_pane_buffer(&mut fs, &cells, 4, 1, 4.0 * cell_w, cell_h, &p);
    let runs: Vec<_> = buf.layout_runs().collect();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].glyphs.len(), 4, "four columns shape to four glyphs");
    for g in runs[0].glyphs {
        let cols = g.x / cell_w;
        assert!(
            (cols - cols.round()).abs() < 1e-3,
            "medium glyph at x={} is off the {cell_w}px grid",
            g.x
        );
    }
}

#[test]
fn semibold_weight_glyphs_snap_to_the_same_cell_advance() {
    // 600 is the shipped default base weight (a thicker body). The fixed-cell
    // invariant must hold for it too, or every row would drift off-grid — the
    // exact failure the letter-spacing correction exists to prevent, keyed on
    // (family, weight, char), so a new weight is a new correction key.
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
    let cells = vec![style(0, 'W'), style(1, 'i'), style(2, 'm'), style(3, '0')];
    let (cell_w, cell_h) = cell_metrics(14.0);
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 600,
    };
    let buf = build_pane_buffer(&mut fs, &cells, 4, 1, 4.0 * cell_w, cell_h, &p);
    let runs: Vec<_> = buf.layout_runs().collect();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].glyphs.len(), 4, "four columns shape to four glyphs");
    for g in runs[0].glyphs {
        let cols = g.x / cell_w;
        assert!(
            (cols - cols.round()).abs() < 1e-3,
            "semibold glyph at x={} is off the {cell_w}px grid",
            g.x
        );
    }
}

#[test]
fn a_heavier_weight_rasterizes_more_ink() {
    // The point of the weight knob: a heavier base weight must actually paint
    // thicker glyphs. Rasterize the same 'M' at Normal (400) and Bold (700)
    // through the swash cache and compare total coverage — heavier = more ink.
    use glyphon::SwashCache;
    let ink = |weight: u16| -> u64 {
        let mut fs = FontSystem::new();
        let mut swash = SwashCache::new();
        let (cell_w, cell_h) = cell_metrics(14.0);
        let cells = vec![CellView {
            col: 0,
            row: 0,
            c: 'M',
            fg: (255, 255, 255),
            bg: (0, 0, 0),
            bold: false,
            italic: false,
        }];
        let p = FontParams {
            font_size: 14.0,
            line_height: cell_h,
            cell_w,
            family: None,
            weight,
        };
        let buf = build_pane_buffer(&mut fs, &cells, 1, 1, cell_w, cell_h, &p);
        let run = buf.layout_runs().next().expect("one run");
        let g = run.glyphs.first().expect("one glyph");
        let phys = g.physical((0.0, 0.0), 1.0);
        // Sum the coverage bytes of the rasterized glyph mask.
        swash
            .get_image(&mut fs, phys.cache_key)
            .as_ref()
            .map(|img| img.data.iter().map(|&b| b as u64).sum())
            .unwrap_or(0)
    };
    let normal = ink(400);
    let bold = ink(700);
    assert!(normal > 0, "the normal glyph should rasterize some ink");
    assert!(
        bold > normal,
        "bold ink ({bold}) should exceed normal ink ({normal})"
    );
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
fn base_weight_is_medium_on_both_appearances() {
    assert_eq!(base_weight(true), 500, "dark themes now read at Medium too");
    assert_eq!(base_weight(false), 500, "light themes read at Medium");
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

#[test]
fn cell_correction_snaps_off_grid_advances_only() {
    let cell_em = 0.6;
    // Advances that already round to one cell need no correction.
    assert_eq!(cell_correction_em(0.6, cell_em), None);
    assert_eq!(
        cell_correction_em(0.55, cell_em),
        None,
        "rounds to 1 anyway"
    );
    assert_eq!(
        cell_correction_em(0.85, cell_em),
        None,
        "rounds to 1 anyway"
    );
    // A narrow glyph (< half a cell) would round to a ZERO advance and shift
    // the whole row left — the reproduced ComicMono `·` bug.
    let ls = cell_correction_em(0.2, cell_em).expect("narrow glyph needs correction");
    assert!(
        (0.2 + ls - cell_em).abs() < 1e-6,
        "corrected advance is exactly one cell"
    );
    // An over-wide width-1 glyph (> 1.5 cells) would round to TWO cells and
    // shift the row right; correction pulls it back to one.
    let ls = cell_correction_em(1.0, cell_em).expect("over-wide glyph needs correction");
    assert!((1.0 + ls - cell_em).abs() < 1e-6);
    // Non-finite advances (GB18030 Bitmap CJK quirk) are left alone.
    assert_eq!(cell_correction_em(f32::INFINITY, cell_em), None);
}

#[test]
fn roster_symbol_glyphs_stay_on_cell_grid() {
    // Repro for the crew-pane roster misalignment: rows mix ASCII with
    // symbol glyphs (marker, middle dot, multiply sign, box pipe, shades,
    // braille spinner). In fonts where a symbol's natural advance is narrow
    // (ComicMono Nerd Font Mono's `·` is < half a cell), cosmic-text's
    // monospace rounding snapped it to a ZERO advance and every glyph after
    // it drifted one cell left. Every width-1 glyph must land exactly on its
    // cell column, whatever family is configured.
    //
    // Font-environment-sensitive: the ComicMono Nerd Font Mono iteration
    // exercises the real repro only where that font is installed; elsewhere
    // it degrades to the fallback path, but the on-grid assertions still
    // hold either way.
    let mk = |col: u16, c: char| CellView {
        col,
        row: 0,
        c,
        fg: (200, 200, 200),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
    };
    let (cell_w, cell_h) = cell_metrics(14.0);
    let chars: Vec<char> =
        "\u{25aa}p \u{2502} \u{00b7} 1\u{00d7} \u{2502} \u{2013} \u{2588}\u{2591} 21% idle \u{2800}\u{2819}"
            .chars()
            .collect();
    for family in [None, Some("ComicMono Nerd Font Mono".to_string())] {
        let mut fs = FontSystem::new();
        let cells: Vec<CellView> = chars
            .iter()
            .enumerate()
            .map(|(i, &c)| mk(i as u16, c))
            .collect();
        let n = cells.len();
        let p = FontParams {
            font_size: 14.0,
            line_height: cell_h,
            cell_w,
            family: family.clone(),
            weight: 400,
        };
        let buf = build_pane_buffer(&mut fs, &cells, n, 1, n as f32 * cell_w, cell_h, &p);
        let glyphs: Vec<_> = buf.layout_runs().flat_map(|r| r.glyphs.to_vec()).collect();
        assert_eq!(
            glyphs.len(),
            n,
            "every cell shapes to one glyph (fam={family:?})"
        );
        for (i, g) in glyphs.iter().enumerate() {
            let got = g.x / cell_w;
            assert!(
                (got - i as f32).abs() < 1e-3,
                "glyph {:?} (U+{:04X}) at col {got:.3}, expected {i} (fam={family:?})",
                chars[i],
                chars[i] as u32
            );
        }
    }
}
