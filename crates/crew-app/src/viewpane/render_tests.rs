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
            image: None,
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
    let _g = crate::app::theme_test_guard();
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
        marks: Vec::new(),
        pictures: Vec::new(),
        blame_w: 0,
        invisibles: false,
        split: false,
        theme: crew_theme::current_id(),
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
    // The cache is keyed on the THEME among other things, and the theme is a
    // global: without this guard a test running in parallel that switches
    // themes invalidates the cache between the two renders below, and this
    // fails intermittently for a reason that has nothing to do with it.
    let _g = crate::app::theme_test_guard();
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

fn searching(text: &str, needle: &str, typing: bool) -> ViewPane {
    use crate::viewpane::detect::Format;
    use crate::viewpane::load::Loaded;
    use crate::viewpane::search::Search;
    use crate::viewpane::LoadState;
    let mut p = ViewPane::open(std::env::temp_dir().join("s.txt"));
    p.state = LoadState::Ready {
        format: Format::Code { lang: "" },
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    p.cache.replace(None);
    let hits = crate::viewpane::search::find_matches(
        &p.lines_for(40)
            .lines
            .iter()
            .map(|l| l.iter().map(|c| c.c).collect::<String>())
            .collect::<Vec<_>>(),
        needle,
    );
    let mut s = Search::new(needle.to_string(), hits);
    s.typing = typing;
    p.search = Some(s);
    p
}

fn row_text(cells: &[crew_render::CellView], row: u16) -> String {
    let mut v: Vec<&crew_render::CellView> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

/// Typing `/` used to be blind: the needle lived only in the pane's state, so
/// a mistyped search and a search with no matches looked identical.
#[test]
fn the_needle_is_drawn_while_it_is_being_typed() {
    let _g = crate::app::theme_test_guard();
    let p = searching("alpha\nbeta\ngamma\n", "al", true);
    let cells = p.cells(40, 6);
    let line = row_text(&cells, 5);
    assert!(line.starts_with("/al"), "{line:?}");
    assert!(line.contains('\u{2588}'), "no caret while typing: {line:?}");
}

/// Confirmed, it reports how much it found — and says so in words rather than
/// leaving you to count the highlights.
#[test]
fn a_confirmed_search_reports_its_tally() {
    let _g = crate::app::theme_test_guard();
    let p = searching("alpha\nbeta\nalpaca\n", "al", false);
    let line = row_text(&p.cells(40, 6), 5);
    assert!(line.starts_with("/al"), "{line:?}");
    assert!(line.contains("2 lines"), "{line:?}");
    let one = searching("alpha\nbeta\n", "beta", false);
    assert!(row_text(&one.cells(40, 6), 5).contains("1 line"));
}

/// A miss says so, in the alarm colour — the one case where the search line
/// has something to correct rather than to report.
#[test]
fn a_search_with_no_matches_says_so() {
    let _g = crate::app::theme_test_guard();
    let p = searching("alpha\nbeta\n", "zzz", false);
    let cells = p.cells(40, 6);
    assert!(row_text(&cells, 5).contains("no matches"));
    let fg = cells.iter().find(|c| c.row == 5).unwrap().fg;
    assert_eq!(fg, crew_theme::theme().bell);
}

/// No search, no line — the row goes back to the document.
#[test]
fn without_a_search_the_last_row_is_content() {
    let _g = crate::app::theme_test_guard();
    let mut p = searching("alpha\nbeta\ngamma\ndelta\nepsilon\nzeta\n", "al", false);
    p.search = None;
    let line = row_text(&p.cells(40, 6), 5);
    assert!(!line.starts_with('/'), "{line:?}");
}

/// The viewer's lines carry BAKED colours — `t.ink`, `text_muted`, the whole
/// `chatink` syntax ladder — decided once when the rendering was built and
/// cached. Nothing invalidated that on a theme change, so `/theme` (and the
/// auto theme flipping at dusk, and the OS switching appearance) left every
/// open viewer wearing the previous palette until something else happened to
/// resize the pane. Dark to light, that is a file drawn in near-white ink on
/// paper: not dimmer, GONE.
#[test]
fn a_theme_switch_repaints_an_open_viewer() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let p = pane_with("let x = 1;\n");
    let ink_of = |p: &ViewPane| {
        p.cells(40, 4)
            .into_iter()
            .find(|c| c.c == 'x')
            .expect("the source line drew")
            .fg
    };
    let dark = ink_of(&p);
    assert_eq!(dark, crew_theme::ThemeId::PaperDark.theme().ink);
    crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
    let light = ink_of(&p);
    assert_eq!(
        light,
        crew_theme::ThemeId::PaperLight.theme().ink,
        "the cached rendering kept the old palette's ink"
    );
    assert_ne!(dark, light);
}
