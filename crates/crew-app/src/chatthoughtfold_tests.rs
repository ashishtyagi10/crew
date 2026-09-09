//! The pane-level path: `Thought` events in, rows out of the card view, the
//! click on the settled row, and the redraw predicate.
use crate::chat::tests::pane;
use crate::chat::ChatPane;
use crate::chatmsgs::card_lines;
use crate::chatmsgs::tests::msg;

const COLS: u16 = 60;
const ROWS: u16 = 30;

fn rows(p: &ChatPane) -> Vec<String> {
    card_lines(&p.visible_messages(), COLS as usize, 0, p.view())
        .iter()
        .map(|l| {
            l.iter()
                .map(|c| c.c)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

/// The absolute pane row whose text contains `needle`.
fn row_of(p: &ChatPane, needle: &str) -> u16 {
    crate::chatplace::placed_lines(p, COLS, ROWS)
        .iter()
        .find(|(_, l)| l.iter().map(|c| c.c).collect::<String>().contains(needle))
        .map(|(r, _)| *r)
        .unwrap_or_else(|| panic!("no row with {needle:?}: {:?}", rows(p)))
}

/// A pane mid-turn: the user asked, coder is thinking, no reply text yet.
fn thinking() -> ChatPane {
    let mut p = pane();
    p.push_capped(msg("user", "why?"));
    p.thoughts
        .absorb("coder", "first, the sidebar\nthen the chart", 1_000);
    p.thoughts.absorb("coder", "\nso: the canvas", 5_200);
    p
}

#[test]
fn a_thought_with_no_card_yet_stands_at_the_end_under_the_agents_badge() {
    let _g = crate::app::motion_test_guard();
    let p = thinking();
    let r = rows(&p);
    let head = r
        .iter()
        .position(|l| l.contains("thinking"))
        .expect("live header");
    assert!(r[head - 1].contains("coder"), "headed by the badge: {r:?}");
    assert_eq!(
        &r[head + 1..],
        [
            "  first, the sidebar",
            "  then the chart",
            "  so: the canvas"
        ]
    );
    assert!(p.thinking_live(), "the shimmer and the clock need frames");
}

#[test]
fn the_streaming_card_seats_the_thought_above_itself() {
    let _g = crate::app::motion_test_guard();
    let mut p = thinking();
    p.absorb_delta("coder".into(), "Short answer:".into());
    let r = rows(&p);
    let head = r
        .iter()
        .position(|l| l.contains("thinking"))
        .expect("live header");
    let card = r
        .iter()
        .position(|l| l.contains("Short answer:"))
        .expect("card");
    assert!(head < card, "thought above the reply it leads to: {r:?}");
    assert_eq!(
        r.iter().filter(|l| l.contains("coder")).count(),
        1,
        "one badge, the card's own header: {r:?}"
    );
}

#[test]
fn the_reply_settling_collapses_the_thought_to_a_row_above_the_card_that_opens_on_click() {
    let _g = crate::app::motion_test_guard();
    let mut p = thinking();
    p.absorb_message(
        "coder \u{2192} user".into(),
        "the canvas".into(),
        "900".into(),
        String::new(),
    );
    assert!(!p.thinking_live(), "nothing live once the reply landed");
    let r = rows(&p);
    let sum = r
        .iter()
        .position(|l| l.contains("thought for"))
        .expect("summary");
    assert_eq!(r[sum], "  \u{25b8} thought for 4.2 s \u{00b7} 48 chars");
    assert!(r[sum + 1].contains("coder"), "right above the reply: {r:?}");
    assert!(
        !r.iter().any(|l| l.contains("sidebar")),
        "collapsed: the text is hidden"
    );

    let row = row_of(&p, "thought for");
    assert!(
        p.fold_target_at(COLS, ROWS, row),
        "the row is a click target"
    );
    assert!(p.toggle_fold_at(COLS, ROWS, row));
    let r = rows(&p);
    let sum = r
        .iter()
        .position(|l| l.contains("thought for"))
        .expect("summary");
    assert!(r[sum].starts_with("  \u{25be}"), "open marker: {r:?}");
    assert_eq!(r[sum + 1], "  first, the sidebar", "{r:?}");
    // The transcript is bottom-anchored: opening it moved the row up.
    let row = row_of(&p, "thought for");
    assert!(p.toggle_fold_at(COLS, ROWS, row), "and back");
    assert!(!rows(&p).iter().any(|l| l.contains("sidebar")));
}

#[test]
fn the_run_ending_abandons_a_live_thought_and_a_new_broker_drops_one() {
    let mut p = thinking();
    p.fold_swarm();
    assert!(!p.thinking_live());
    assert_eq!(p.thoughts.settled.len(), 1, "kept, as an orphan block");
    assert!(p.thoughts.settled[0].anchor.is_none());

    let mut p = thinking();
    p.reset_broker_state();
    assert!(
        !p.thinking_live(),
        "a fresh broker cannot finish the old thought"
    );
    assert!(p.thoughts.settled.is_empty());
}

#[test]
fn the_pane_asks_for_frames_only_while_a_thought_is_live() {
    let p = pane();
    assert!(!p.thinking_live(), "an idle pane never animates for this");
    let mut p = thinking();
    assert!(p.thinking_live());
    p.absorb_thought("coder".into(), "more".into());
    assert!(p.thinking_live());
    p.absorb_message("coder".into(), "ok".into(), "1".into(), String::new());
    assert!(!p.thinking_live());
}
