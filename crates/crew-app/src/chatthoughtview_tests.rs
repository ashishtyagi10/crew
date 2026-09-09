use super::*;
use crate::chatthought::{Live, ThoughtBlock, LIVE_ROWS, THOUGHT_ROWS};

fn text(l: &CardLine) -> String {
    l.iter()
        .map(|c| c.c)
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn texts(lines: &[CardLine]) -> Vec<String> {
    lines.iter().map(text).collect()
}

fn block(text: &str, ms: u64) -> ThoughtBlock {
    ThoughtBlock {
        agent: "coder".into(),
        anchor: Some("500".into()),
        text: text.into(),
        ms,
        expanded: false,
    }
}

#[test]
fn the_live_block_shows_the_header_then_only_the_last_four_lines() {
    let _g = crate::app::motion_test_guard();
    let l = Live {
        agent: "coder".into(),
        text: "one\ntwo\nthree\nfour\nfive\nsix".into(),
        since_ms: 1_000,
        last_ms: 4_000,
    };
    let rows = live_lines(&l, 4_500, 60);
    assert_eq!(rows.len(), 1 + LIVE_ROWS);
    assert_eq!(text(&rows[0]), "  \u{2234} thinking \u{00b7} 3s");
    assert_eq!(
        texts(&rows[1..]),
        ["  three", "  four", "  five", "  six"],
        "the tail, not the head"
    );
    assert!(
        rows[1].iter().all(|c| c.italic),
        "the working reads in italic"
    );
    let counting = live_lines(&l, 0, 60);
    assert_eq!(
        counting.len(),
        rows.len(),
        "the counting pass sees the same rows"
    );
    assert_eq!(
        text(&counting[0]),
        "  \u{2234} thinking",
        "no clock at now = 0"
    );
}

#[test]
fn the_settled_row_says_how_long_and_how_much_and_opens_to_the_capped_text() {
    let _g = crate::app::motion_test_guard();
    let mut b = block(&"w".repeat(812), 4_200);
    assert_eq!(
        text(&summary(&b, 60)),
        "  \u{25b8} thought for 4.2 s \u{00b7} 812 chars"
    );
    assert_eq!(block_lines(&b, 60).len(), 1, "collapsed: the row alone");
    b.expanded = true;
    let rows = block_lines(&b, 60);
    assert_eq!(
        text(&rows[0]),
        "  \u{25be} thought for 4.2 s \u{00b7} 812 chars"
    );
    assert!(rows.len() > 2 && rows.len() <= 1 + THOUGHT_ROWS);
    let instant = block("hm", 0);
    assert_eq!(
        text(&summary(&instant, 60)),
        "  \u{25b8} thought \u{00b7} 2 chars",
        "a thought that arrived whole has no span to report"
    );
}

#[test]
fn a_long_thought_is_capped_at_forty_rows_with_a_count_of_the_rest() {
    let _g = crate::app::motion_test_guard();
    let long: Vec<String> = (1..=60).map(|i| format!("line {i}")).collect();
    let mut b = block(&long.join("\n"), 1_000);
    b.expanded = true;
    let rows = block_lines(&b, 60);
    assert_eq!(rows.len(), 1 + THOUGHT_ROWS);
    assert_eq!(text(&rows[1]), "  line 1");
    assert_eq!(text(rows.last().unwrap()), "  \u{2026} +21 lines");
}
