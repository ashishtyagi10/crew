use super::*;
use crate::chat::ChatPane;
use crew_plugin::Plugin;

fn pane() -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

#[test]
fn stop_bypasses_the_queue_regardless_of_spacing() {
    assert!(is_stop("/stop"));
    assert!(is_stop("  /stop  "));
    assert!(is_stop("/stop #2"));
    assert!(!is_stop("/stopwatch"));
    assert!(!is_stop("hello /stop"));
}

#[test]
fn queued_rows_is_zero_when_empty_one_when_not() {
    let mut p = pane();
    assert_eq!(queued_rows(&p), 0);
    p.queued.push_back("hi".into());
    assert_eq!(queued_rows(&p), 1);
    p.queued.push_back("there".into());
    assert_eq!(queued_rows(&p), 1, "one row regardless of depth");
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
    let cells = indicator_cells(&p, 80, 7);
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
    assert!(indicator_cells(&p, 80, 7).is_empty());
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
