use crate::viewpane::ViewPane;

fn pane_with(text: &str) -> ViewPane {
    use crate::viewpane::detect::Format;
    use crate::viewpane::load::Loaded;
    use crate::viewpane::LoadState;
    let mut p = ViewPane::open(std::env::temp_dir().join("r.txt"));
    p.state = LoadState::Ready {
        format: Format::Code { lang: "" },
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
        },
    };
    p
}

#[test]
fn cells_fit_inside_the_grid() {
    let p = pane_with("one\ntwo\nthree\n");
    for c in p.cells(20, 3) {
        assert!(
            c.col < 20 && c.row < 3,
            "cell {:?},{:?} escaped",
            c.col,
            c.row
        );
    }
}

#[test]
fn a_zero_sized_grid_draws_nothing_and_does_not_panic() {
    assert!(pane_with("x\n").cells(0, 0).is_empty());
}

// `cells_fit_inside_the_grid` only checks `c.col < cols`, which is true even
// for a wide glyph whose col + width overruns the edge — its fixture is
// ASCII-only anyway, so char_w is always 1 there and the case never arises.
// Inject a CardLine directly (bypassing `lines::for_state`, which already
// wraps by display width and so never produces this shape on its own) so a
// double-width glyph lands with one column of room left: four 1-wide chars
// then a 2-wide CJK glyph in a 5-column grid. A naive char-count placement
// emits it at col 4 (4 < 5, so `cells_fit_inside_the_grid`-style checks pass)
// while its true extent reaches column 5, one past the last valid column.
#[test]
fn a_double_width_glyph_never_straddles_the_grid_edge() {
    use crate::chatbody::plain;
    use crate::viewpane::ViewCache;
    let ink = crew_theme::theme().ink;
    let line: Vec<_> = "aaaa\u{4e2d}"
        .chars()
        .map(|c| plain(c, ink, false))
        .collect();
    let p = pane_with("");
    p.cache.replace(Some(ViewCache {
        cols: 5,
        raw: false,
        lines: vec![line],
    }));
    for c in p.cells(5, 1) {
        let w = crate::chatwidth::char_w(c.c) as u16;
        assert!(
            c.col + w <= 5,
            "cell {:?} at col {} width {} overruns a 5-col grid",
            c.c,
            c.col,
            w
        );
    }
}

#[test]
fn clamp_scroll_pulls_a_wild_offset_back_to_the_last_page() {
    // `cells` clamps what it DRAWS to the last full page, but that alone
    // isn't enough — the STORED `scroll` offset must be clamped too, or a
    // later scroll-up tick starts from the wild value and stays dead until
    // it walks all the way back down to content. (The Shift+End lesson.)
    let mut p = pane_with("a\nb\nc\nd\n");
    p.scroll = 9_000;
    p.clamp_scroll(20, 2);
    assert!(p.scroll <= 3, "offset clamped to content, got {}", p.scroll);
}

// Proves REUSE, not merely that a cache exists: mutating unrelated state
// and re-rendering at the same width must not rebuild. The obvious version
// of this test — render twice, assert the cache holds width 30 — passes
// even against an always-rebuild implementation, because a rebuilt cache
// holds width 30 too.
#[test]
fn the_cache_survives_an_unrelated_state_mutation_at_the_same_width() {
    let mut p = pane_with("first\n");
    let _ = p.cells(30, 5);
    if let crate::viewpane::LoadState::Ready { loaded, .. } = &mut p.state {
        loaded.text = "second\n".into();
    }
    let cells = p.cells(30, 5);
    let row: String = cells.iter().filter(|c| c.row == 0).map(|c| c.c).collect();
    assert!(
        row.contains('f') && !row.contains("second"),
        "a reused cache still shows the original text, got {row:?}"
    );
}
