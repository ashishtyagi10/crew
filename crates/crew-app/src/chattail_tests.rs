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
fn tail_appears_when_several_agents_stream_at_once() {
    let p = streaming_pane(3); // the newest is not the one drawing the eye
    assert_eq!(tail_rows(&p, 80), TAIL_ROWS);
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
