use super::*;

fn streaming_pane(n: usize) -> crate::chat::ChatPane {
    let mut p = crate::chat::tests::pane();
    for i in 0..n {
        p.absorb_delta(format!("agent{i}"), "some streamed text".into());
    }
    p
}

#[test]
fn no_tail_when_the_single_growing_card_is_visible() {
    let p = streaming_pane(1); // scroll == 0, one agent → the card is on screen
    assert_eq!(tail_rows(&p, 80), 0);
}

#[test]
fn tail_appears_when_scrolled_away_from_the_live_bottom() {
    let mut p = streaming_pane(1);
    p.scroll = 5;
    assert_eq!(tail_rows(&p, 80), TAIL_ROWS);
}

#[test]
fn no_tail_from_parallelism_alone_at_the_live_bottom() {
    // `chatplace::window` is bottom-anchored: at scroll == 0 it always ends
    // at the newest line, whatever the row budget, so the newest streaming
    // card (last in `visible_messages()`) is always on screen no matter how
    // many agents are streaming. Several concurrent agents is NOT, on its
    // own, a reason to show the tail — only scrolling away is.
    let p = streaming_pane(3);
    assert_eq!(tail_rows(&p, 80), 0);
}

#[test]
fn no_tail_without_any_streaming_card() {
    let mut p = crate::chat::tests::pane();
    p.scroll = 5;
    assert_eq!(tail_rows(&p, 80), 0);
}

#[test]
fn tail_follows_the_most_recently_updated_agent() {
    let mut p = streaming_pane(2);
    p.scroll = 5;
    p.absorb_delta("agent0".into(), " NEWEST".into());
    let cells = tail_cells(&p, 80, 0);
    let drawn: String = cells.iter().map(|c| c.c).collect();
    assert!(
        drawn.contains("NEWEST"),
        "the tail tracks the last agent to produce text, not the first to start: {drawn:?}"
    );
}

/// Regression (Finding 1): with 2+ agents streaming and `scroll == 0`, the
/// bottom-anchored transcript already shows the newest card's tail end —
/// budgeting rows for `chattail` on top of that used to redraw the same text
/// a second time, immediately above it. This drives the WHOLE pane through
/// `ChatPane::cells` (not the isolated `tail_rows`/`tail_cells` units, which
/// can't see the transcript the tail sits above) so the two surfaces'
/// combined output is what's actually checked.
#[test]
fn streamed_text_never_duplicates_with_several_agents_at_the_live_bottom() {
    let mut p = crate::chat::tests::pane();
    p.absorb_delta("agent0".into(), "zeromarkerxyz".into());
    p.absorb_delta("agent1".into(), "onemarkerxyz".into());
    assert_eq!(p.scroll, 0, "fixture must start at the live bottom");

    // Generous, roomy panes — big enough that both one-line cards comfortably
    // fit the transcript budget, so both markers are expected on screen
    // exactly once. (Cramped panes legitimately scroll agent0's older card
    // off entirely; that's `chatplace::window`'s ordinary behaviour, not the
    // duplication this test targets, so this deliberately doesn't sweep down
    // into that regime.)
    for rows in [16u16, 20, 24] {
        for cols in [40u16, 60, 80] {
            let cells = p.cells(cols, rows);
            let drawn: String = cells.iter().map(|c| c.c).collect();
            assert_eq!(
                drawn.matches("zeromarkerxyz").count(),
                1,
                "agent0's text duplicated (or missing) at cols={cols} rows={rows}: {drawn:?}"
            );
            assert_eq!(
                drawn.matches("onemarkerxyz").count(),
                1,
                "agent1's text duplicated (or missing) at cols={cols} rows={rows}: {drawn:?}"
            );
        }
    }
}

// -- Finding 2 follow-up: `tail_cells` renders through `chatbody::body_lines`
// (the same pipeline the transcript itself uses), so these confirm the
// display-width edge cases that pipeline is expected to already handle
// correctly are not somehow broken by the tail's own row-slicing / recolour
// step on top of it. ---------------------------------------------------

#[test]
fn empty_streaming_card_does_not_panic() {
    let mut p = crate::chat::tests::pane();
    p.absorb_delta("agent0".into(), String::new());
    p.scroll = 5;
    assert_eq!(tail_rows(&p, 80), TAIL_ROWS);
    // Must not panic, and whatever it draws must stay in bounds.
    let cells = tail_cells(&p, 80, 0);
    assert!(cells.iter().all(|c| c.col < 80), "cell escaped cols=80");
}

#[test]
fn a_single_word_longer_than_cols_hard_breaks_instead_of_overflowing() {
    let mut p = crate::chat::tests::pane();
    p.absorb_delta("agent0".into(), "x".repeat(100));
    p.scroll = 5;
    let cols = 20u16;
    let cells = tail_cells(&p, cols, 0);
    assert!(
        !cells.is_empty(),
        "expected the hard-broken word to draw something"
    );
    for c in &cells {
        assert!(
            c.col < cols,
            "cell at col {} escaped the {cols}-column budget",
            c.col
        );
    }
}

#[test]
fn wide_cjk_glyphs_never_overflow_the_column_budget() {
    let mut p = crate::chat::tests::pane();
    p.absorb_delta("agent0".into(), "\u{6f22}\u{5b57}".repeat(30));
    p.scroll = 5;
    let cols = 20u16;
    let cells = tail_cells(&p, cols, 0);
    assert!(!cells.is_empty());
    // Group by row and check the summed display width never exceeds `cols`.
    use std::collections::BTreeMap;
    let mut by_row: BTreeMap<u16, u16> = BTreeMap::new();
    for c in &cells {
        *by_row.entry(c.row).or_default() += crate::chatwidth::char_w(c.c) as u16;
    }
    for (row, w) in by_row {
        assert!(w <= cols, "row {row} exceeds width budget ({w} > {cols})");
    }
}
