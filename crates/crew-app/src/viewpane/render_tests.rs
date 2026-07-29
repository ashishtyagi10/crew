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

#[test]
fn clamp_scroll_pulls_a_wild_offset_back_to_the_last_page() {
    // window_top clamps the VIEW; the stored offset must be clamped too or
    // every later scroll-up tick is dead. (The Shift+End lesson.)
    let mut p = pane_with("a\nb\nc\nd\n");
    p.scroll = 9_000;
    p.clamp_scroll(20, 2);
    assert!(p.scroll <= 3, "offset clamped to content, got {}", p.scroll);
}

#[test]
fn the_cache_is_reused_across_frames_at_one_width() {
    let p = pane_with("a\nb\n");
    let _ = p.cells(30, 5);
    let before = p.cache.borrow().as_ref().map(|c| c.cols);
    let _ = p.cells(30, 5);
    assert_eq!(before, Some(30), "same width keeps the cache");
}

// The assertion above only proves a cache was BUILT once — `before` is read
// after the first call, so it passes even if the second `cells()` rebuilds
// the cache from scratch (a "stale" check that is always true would still
// leave `cache.cols == 30` after the rebuild). To actually observe reuse
// rather than mere presence, mutate the pane's loaded text directly —
// bypassing `reload()`, which is the only path that is supposed to
// invalidate the cache — and confirm a second `cells()` call at the SAME
// width still renders the ORIGINAL text. A correct cache reuses the
// stale-but-untouched lines; a cache that rebuilds on every call would pick
// up the mutation immediately.
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
