//! The indicator as it is drawn. The rules it draws from — what is waiting
//! and how many rows that needs — are tested beside them in `chatqueue`.
use super::*;
use crate::chat::ChatPane;
use crate::chatqueue::{queued_rows, SHOWN};
use crew_plugin::Plugin;

fn pane() -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

/// The rows the indicator actually drew, top to bottom.
fn drawn(p: &ChatPane, cols: u16, rows: u16) -> Vec<String> {
    let cells = indicator_cells_at(p, cols, 5, rows, 0);
    let mut by_row: std::collections::BTreeMap<u16, Vec<(u16, char)>> = Default::default();
    for c in &cells {
        by_row.entry(c.row).or_default().push((c.col, c.c));
    }
    by_row
        .into_values()
        .map(|mut r| {
            r.sort();
            r.into_iter()
                .map(|(_, c)| c)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

#[test]
fn indicator_text_is_none_when_empty() {
    let p = pane();
    assert_eq!(indicator_text(&p, 0), None);
}

#[test]
fn indicator_text_singular_and_plural() {
    let mut p = pane();
    p.queued.push_back("a".into());
    let text = indicator_text(&p, 0).expect("one queued");
    assert!(text.contains("1 message queued"), "got: {text}");
    assert!(text.contains("sends when the crew is idle"), "got: {text}");

    p.queued.push_back("b".into());
    let text = indicator_text(&p, 0).expect("two queued");
    assert!(text.contains("2 messages queued"), "got: {text}");
}

#[test]
fn indicator_cells_render_the_count_at_the_given_row() {
    let mut p = pane();
    p.queued.push_back("a".into());
    p.queued.push_back("b".into());
    let cells = indicator_cells(&p, 80, 7, 1);
    assert!(!cells.is_empty());
    assert!(cells.iter().all(|c| c.row == 7), "all on the given row");
    let text: String = {
        let mut row: Vec<(u16, char)> = cells.iter().map(|c| (c.col, c.c)).collect();
        row.sort();
        row.into_iter().map(|(_, c)| c).collect()
    };
    assert!(text.contains("2 messages queued"), "got: {text}");
}

#[test]
fn indicator_cells_empty_when_queue_empty() {
    let p = pane();
    assert!(indicator_cells(&p, 80, 7, 4).is_empty());
}

/// On main the indicator opened with a fixed `⧗`. The hourglass turns now:
/// the two frames alternate every `HOURGLASS_MS` while anything is queued,
/// and Off holds the first frame still.
#[test]
fn the_hourglass_turns_every_period_and_stands_still_at_off() {
    let _g = crate::app::motion_test_guard();
    let _plain = crate::glyphs::force(false);
    use crate::motion::MotionLevel::{Full, Off};
    let (a, b) = (hourglass(0, Full), hourglass(HOURGLASS_MS, Full));
    assert_eq!((a, b), ('\u{29d7}', '\u{29d6}'), "the pair, in order");
    assert_eq!(hourglass(2 * HOURGLASS_MS, Full), a, "and round again");
    assert_eq!(
        hourglass(HOURGLASS_MS - 1, Full),
        a,
        "a frame lasts a period"
    );
    for now in [0, HOURGLASS_MS, 3 * HOURGLASS_MS + 7] {
        assert_eq!(hourglass(now, Off), a, "Off is static at now={now}");
    }
    // The text carries the frame, so the drawn row turns with it.
    crate::motion::set_level(Full);
    let mut p = pane();
    p.queued.push_back("a".into());
    let first = indicator_text(&p, 0).unwrap();
    let second = indicator_text(&p, HOURGLASS_MS).unwrap();
    assert!(first.starts_with('\u{29d7}') && second.starts_with('\u{29d6}'));
    assert_eq!(first[3..], second[3..], "only the glass changes");
}

/// A half-width tile marks the cut and keeps a column of air at the edge.
#[test]
fn the_indicator_marks_its_cut_on_a_narrow_pane() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.queued.push_back("hi".into());
    let cells = indicator_cells_at(&p, 30, 0, 1, 0);
    let mut v: Vec<_> = cells.iter().collect();
    v.sort_by_key(|c| c.col);
    let s: String = v.iter().map(|c| c.c).collect();
    assert!(s.ends_with('\u{2026}'), "{s:?}");
    assert!(cells.iter().all(|c| c.col < 30), "{s:?}");
    let wide = |p: &ChatPane| -> String {
        indicator_cells_at(p, 100, 0, 1, 0)
            .iter()
            .map(|c| c.c)
            .collect()
    };
    assert!(
        wide(&p).ends_with("backspace takes one back"),
        "{:?}",
        wide(&p)
    );
    // With something typed, backspace is deleting THAT, and a hint you
    // cannot act on is exactly the noise this surface avoids.
    p.input.push_str("half a thought");
    assert!(wide(&p).ends_with("idle"), "{:?}", wide(&p));
}

#[test]
fn the_queue_says_what_is_waiting_not_only_how_much() {
    // The whole point: a message typed behind a long run used to be invisible
    // until it sent itself.
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.queued.push_back("fix the nav crash".into());
    p.queued.push_back("then run the tests".into());
    let rows = drawn(&p, 80, queued_rows(&p));
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert!(rows[0].contains("2 messages queued"), "{rows:?}");
    assert!(rows[1].ends_with("1. fix the nav crash"), "{rows:?}");
    assert!(rows[2].ends_with("2. then run the tests"), "{rows:?}");
}

#[test]
fn a_deep_queue_lists_the_next_few_and_counts_the_rest() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    for i in 1..=7 {
        p.queued.push_back(format!("task {i}"));
    }
    let rows = drawn(&p, 80, queued_rows(&p));
    assert_eq!(rows.len(), 1 + SHOWN + 1, "{rows:?}");
    assert!(rows[1].ends_with("1. task 1"), "{rows:?}");
    assert!(rows[SHOWN].ends_with("3. task 3"), "{rows:?}");
    assert!(rows[SHOWN + 1].ends_with("\u{2026} +4 more"), "{rows:?}");
}

#[test]
fn a_pane_with_one_row_to_spare_still_says_how_many() {
    // The degradation the design rests on: `chatplace::grants` clamps every
    // surface to what is left, and the row it can always afford is the count.
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.queued.push_back("a".into());
    p.queued.push_back("b".into());
    let rows = drawn(&p, 80, 1);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(rows[0].contains("2 messages queued"), "{rows:?}");
    // And a grant of zero draws nothing at all rather than overrunning.
    assert!(indicator_cells_at(&p, 80, 5, 0, 0).is_empty());
}

#[test]
fn a_long_queued_message_is_cut_with_a_mark_inside_the_pane() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.queued.push_back("x".repeat(200));
    let cells = indicator_cells_at(&p, 40, 5, queued_rows(&p), 0);
    assert!(cells.iter().all(|c| c.col < 40), "drew outside the pane");
    let last: String = drawn(&p, 40, queued_rows(&p)).pop().unwrap();
    assert!(last.ends_with('\u{2026}'), "{last:?}");
}

#[test]
fn the_list_is_indented_under_the_line_that_counts_it() {
    // Otherwise the summary and its contents read as equal notices.
    let _g = crate::app::theme_test_guard();
    let mut p = pane();
    p.queued.push_back("a".into());
    let cells = indicator_cells_at(&p, 80, 5, queued_rows(&p), 0);
    let left = |row: u16| cells.iter().filter(|c| c.row == row).map(|c| c.col).min();
    assert_eq!(left(5), Some(1), "the summary keeps the canvas inset");
    assert_eq!(left(6), Some(3), "the list sits under it");
}
