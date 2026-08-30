use super::*;

fn params(family: Option<String>) -> FontParams {
    FontParams {
        font_size: 14.0,
        line_height: 17.5,
        cell_w: 14.0 * 0.6,
        family,
        weight: 400,
        smooth: 0,
        gamma: 0,
        dark: true,
        body: ((255, 255, 255), (0, 0, 0)),
    }
}

#[test]
fn cell_metrics_larger_font_gives_larger_dimensions() {
    let small = cell_metrics(12.0, CELL_H_RATIO);
    let large = cell_metrics(24.0, CELL_H_RATIO);
    assert!(large.0 > small.0, "cell_w should grow with font size");
    assert!(large.1 > small.1, "cell_h should grow with font size");
    assert_eq!(large.1, 24.0 * 1.25, "cell_h is 1.25× font size");
}

#[test]
fn cell_metrics_height_is_125_percent() {
    assert_eq!(cell_metrics(16.0, CELL_H_RATIO).1, 20.0);
}

#[test]
fn cell_metrics_are_family_independent_and_whole_pixel() {
    // The whole point of the fixed box: the same size gives the same cell no
    // matter which family the user picks — snapped to whole physical pixels
    // (14 × 0.6 = 8.4 → 8, 14 × 1.25 = 17.5 → 18) so glyphs never smear.
    assert_eq!(cell_metrics(14.0, CELL_H_RATIO), (8.0, 18.0));
    let (w, h) = cell_metrics(28.0, CELL_H_RATIO); // 2x display
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
        ..Default::default()
    };
    let mut fs = crate::embedfont::font_system();
    let cells = vec![
        style(0, 'W', true),
        style(1, 'i', true),
        style(2, 'm', false),
        style(3, '0', true),
    ];
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 400,
        smooth: 0,
        gamma: 0,
        dark: true,
        body: ((255, 255, 255), (0, 0, 0)),
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
        ..Default::default()
    };
    let mut fs = crate::embedfont::font_system();
    let cells = vec![style(0, 'W'), style(1, 'i'), style(2, 'm'), style(3, '0')];
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 500,
        smooth: 0,
        gamma: 0,
        dark: true,
        body: ((255, 255, 255), (0, 0, 0)),
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
        ..Default::default()
    };
    let mut fs = crate::embedfont::font_system();
    let cells = vec![style(0, 'W'), style(1, 'i'), style(2, 'm'), style(3, '0')];
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 600,
        smooth: 0,
        gamma: 0,
        dark: true,
        body: ((255, 255, 255), (0, 0, 0)),
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
        let mut fs = crate::embedfont::font_system();
        let mut swash = SwashCache::new();
        let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
        let cells = vec![CellView {
            col: 0,
            row: 0,
            c: 'M',
            fg: (255, 255, 255),
            bg: (0, 0, 0),
            bold: false,
            italic: false,
            ..Default::default()
        }];
        let p = FontParams {
            font_size: 14.0,
            line_height: cell_h,
            cell_w,
            family: None,
            weight,
            smooth: 0,
            gamma: 0,
            dark: true,
            body: ((255, 255, 255), (0, 0, 0)),
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
    let mut fs = crate::embedfont::font_system();
    let cells = vec![
        CellView {
            col: 0,
            row: 0,
            c: 'h',
            fg: (200, 200, 200),
            bg: (0, 0, 0),
            bold: true,
            italic: false,
            ..Default::default()
        },
        CellView {
            col: 1,
            row: 0,
            c: 'i',
            fg: (10, 20, 30),
            bg: (0, 0, 0),
            bold: false,
            italic: true,
            ..Default::default()
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
            ..Default::default()
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
    let mut fs = crate::embedfont::font_system();
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
        ..Default::default()
    };
    let mut fs = crate::embedfont::font_system();
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
    let mut fs = crate::embedfont::font_system();
    // A cell beyond cols/rows must be dropped without panicking.
    let cells = vec![CellView {
        col: 9,
        row: 9,
        c: 'z',
        fg: (1, 1, 1),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
        ..Default::default()
    }];
    let _ = build_pane_buffer(&mut fs, &cells, 2, 2, 16.0, 32.0, &params(None));
}

#[test]
fn cell_correction_snaps_off_grid_advances_only() {
    let cell_em = 0.6;
    // Advances that already round to one cell need no correction.
    assert_eq!(cell_correction_em(0.6, cell_em, 1), None);
    assert_eq!(
        cell_correction_em(0.55, cell_em, 1),
        None,
        "rounds to 1 anyway"
    );
    assert_eq!(
        cell_correction_em(0.85, cell_em, 1),
        None,
        "rounds to 1 anyway"
    );
    // A narrow glyph (< half a cell) would round to a ZERO advance and shift
    // the whole row left — the reproduced ComicMono `·` bug.
    let ls = cell_correction_em(0.2, cell_em, 1).expect("narrow glyph needs correction");
    assert!(
        (0.2 + ls - cell_em).abs() < 1e-6,
        "corrected advance is exactly one cell"
    );
    // An over-wide width-1 glyph (> 1.5 cells) would round to TWO cells and
    // shift the row right; correction pulls it back to one.
    let ls = cell_correction_em(1.0, cell_em, 1).expect("over-wide glyph needs correction");
    assert!((1.0 + ls - cell_em).abs() < 1e-6);
    // Non-finite advances (GB18030 Bitmap CJK quirk) are left alone.
    assert_eq!(cell_correction_em(f32::INFINITY, cell_em, 1), None);

    // A full-width glyph was placed in TWO columns and is measured against
    // two. The CJK face the fallback reaches at weight 500 advances ~1.1
    // cells, which the monospace rounding snaps to ONE — a two-column
    // character drawn over its neighbour.
    assert_eq!(cell_correction_em(1.2, cell_em, 2), None, "rounds to 2");
    let ls = cell_correction_em(0.66, cell_em, 2).expect("a narrow wide glyph needs correction");
    assert!(
        (0.66 + ls - 2.0 * cell_em).abs() < 1e-6,
        "corrected advance is exactly two cells"
    );
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
    // exercises the real repro only where that font is installed. Elsewhere
    // it is SKIPPED: requesting an absent family makes fontdb substitute
    // per-glyph fallbacks whose advances may round to 2 cells (bare CI
    // runners proved the grid does NOT survive that) — a state production
    // can't reach, since resolve_family/font_pool never apply a family that
    // isn't installed. The None iteration covers the bare-fallback path.
    let mk = |col: u16, c: char| CellView {
        col,
        row: 0,
        c,
        fg: (200, 200, 200),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
        ..Default::default()
    };
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let chars: Vec<char> =
        "\u{25aa}p \u{2502} \u{00b7} 1\u{00d7} \u{2502} \u{2013} \u{2588}\u{2591} 21% idle \u{2800}\u{2819}"
            .chars()
            .collect();
    for family in [None, Some("ComicMono Nerd Font Mono".to_string())] {
        let mut fs = crate::embedfont::font_system();
        if let Some(fam) = &family {
            let installed = fs
                .db()
                .faces()
                .any(|face| face.families.iter().any(|(name, _)| name == fam));
            if !installed {
                continue; // see the doc comment — unreachable in production
            }
        }
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
            smooth: 0,
            gamma: 0,
            dark: true,
            body: ((255, 255, 255), (0, 0, 0)),
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

/// Every shaped glyph lands somewhere finite, whatever face the fallback
/// chain reaches for.
///
/// macOS ships `GB18030 Bitmap`, a bitmap-only face whose em is zero, so
/// every metric scaled out of it is infinite. One Japanese character in an
/// agent's reply was enough: the glyph's advance came back `inf`, every `x`
/// after it on the row followed, and the shaper's subpixel binning overflowed
/// an `f32 as i32` and took the frame down. Nothing in crew could correct it
/// — letter-spacing is ADDED to an advance — so `banish_broken_faces` drops
/// the face and re-shapes. This test only fails on a machine that has such a
/// font installed, which is exactly the machine it has to pass on.
#[test]
fn a_fallback_face_with_broken_metrics_never_reaches_the_layout() {
    let mut fs = crate::embedfont::font_system();
    let (cell_w, cell_h) = cell_metrics(14.0, CELL_H_RATIO);
    let mut p = params(None);
    p.cell_w = cell_w;
    p.line_height = cell_h;
    // Han, hiragana, katakana, hangul and an emoji — the scripts the embedded
    // family does not cover, so every one of them shapes through fallback.
    let text = "日本語のカナと한글と🙂";
    let cells: Vec<CellView> = text
        .chars()
        .enumerate()
        .map(|(i, c)| CellView {
            col: i as u16,
            row: 0,
            c,
            fg: (200, 200, 200),
            bg: (0, 0, 0),
            ..Default::default()
        })
        .collect();
    let cols = cells.len();
    let buf = build_pane_buffer(&mut fs, &cells, cols, 1, cols as f32 * cell_w, cell_h, &p);
    for run in buf.layout_runs() {
        for g in run.glyphs {
            assert!(
                g.x.is_finite() && g.w.is_finite(),
                "{:?} shaped to x={} w={}",
                &run.text[g.start..g.end],
                g.x,
                g.w
            );
        }
    }
}

/// A full-width character occupies TWO columns of the grid, and it has to
/// advance exactly two — at every weight the app can be wearing.
///
/// Two separate things broke this. The column a wide glyph's second half sits
/// in carries no cell of its own (the terminal drops alacritty's spacer, and
/// every in-app widget places one `CellView` per character), and the blank
/// `fill_rich_text` put there gave a two-cell character a THREE-cell advance.
/// Underneath that, `set_monospace_width` snaps to the nearest cell multiple
/// and the CJK face the fallback reaches at weight 500 — the weight every
/// light theme uses — advances under 1.5 cells, so it snapped to ONE. The two
/// cancelled out into a row that added up while every glyph sat over its
/// neighbour; with only the first fixed, a line of Japanese came out crammed.
#[test]
fn a_wide_glyph_advances_exactly_two_cells_at_every_weight() {
    // Both axes matter. The snap is to the nearest CELL multiple, so it turns
    // on the cell-to-em ratio, which the whole-pixel rounding in
    // `cell_metrics` moves with the font size: at 14px a cell is 0.571em and
    // the CJK advance rounds to two on its own; at 13px it is 0.615em and
    // rounds to one. Weight picks the face, and the two faces do not agree.
    for (size, weight) in [(13.0f32, 400u16), (13.0, 500), (13.0, 600), (26.0, 500)] {
        let mut fs = crate::embedfont::font_system();
        let (cell_w, cell_h) = cell_metrics(size, CELL_H_RATIO);
        let mut p = params(None);
        p.font_size = size;
        p.cell_w = cell_w;
        p.line_height = cell_h;
        p.weight = weight;
        let at = |col: u16, c: char| CellView {
            col,
            row: 0,
            c,
            fg: (200, 200, 200),
            bg: (0, 0, 0),
            ..Default::default()
        };
        // 日 at 0-1, 本 at 2-3, then an ASCII marker at 4.
        let cells = vec![at(0, '\u{65e5}'), at(2, '\u{672c}'), at(4, 'x')];
        let cols = 6;
        let buf = build_pane_buffer(&mut fs, &cells, cols, 1, cols as f32 * cell_w, cell_h, &p);
        let run = buf.layout_runs().next().expect("one row");
        let marker = run
            .glyphs
            .iter()
            .find(|g| run.text[g.start..g.end].starts_with('x'))
            .expect("the marker shaped");
        assert_eq!(
            (marker.x / cell_w).round() as i32,
            4,
            "at {size}px weight {weight} the marker is in column 4; it shaped at x={} ({} cells)",
            marker.x,
            marker.x / cell_w
        );
    }
}
