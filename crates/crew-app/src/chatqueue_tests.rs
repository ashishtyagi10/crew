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
    assert_eq!(indicator_text(&p), None);
}

#[test]
fn indicator_text_singular_and_plural() {
    let mut p = pane();
    p.queued.push_back("a".into());
    let text = indicator_text(&p).expect("one queued");
    assert!(text.contains("1 message queued"), "got: {text}");
    assert!(text.contains("sends when the crew is idle"), "got: {text}");

    p.queued.push_back("b".into());
    let text = indicator_text(&p).expect("two queued");
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
