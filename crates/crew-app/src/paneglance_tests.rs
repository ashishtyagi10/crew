use super::*;
use crate::todopane::item::TodoItem;

fn item(title: &str, done: bool, due_ms: Option<u64>) -> TodoItem {
    TodoItem {
        id: 1,
        title: title.into(),
        done,
        done_ms: None,
        project: None,
        assignee: None,
        due_ms,
        due_has_time: false,
        created_ms: 0,
        notified: false,
        run: None,
    }
}

/// The list says what is open and what is late — the two reasons to go back
/// to it. An empty list says that instead of `0 open`.
#[test]
fn a_todo_list_counts_what_is_open_and_late() {
    let now = crate::chattime::unix_now_ms();
    let pane = |items: Vec<TodoItem>| of(&PaneContent::Todo(crate::todopane::test_pane(items)));
    assert_eq!(pane(Vec::new()).as_deref(), Some("nothing open"));
    assert_eq!(
        pane(vec![item("a", false, None), item("b", true, None)]).as_deref(),
        Some("1 open")
    );
    assert_eq!(
        pane(vec![
            item("a", false, Some(now - 1000)),
            item("b", false, None),
        ])
        .as_deref(),
        Some("1 overdue \u{b7} 2 open")
    );
}

/// A narrow thumbnail keeps the late count, the one number worth restoring
/// the pane for, and draws it in the attention ink while the rest stays
/// muted. With the open count first the clip dropped `· 1 overdue` whole.
#[test]
fn a_narrow_thumbnail_keeps_the_overdue_count_loud() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let now = crate::chattime::unix_now_ms();
    let mut items = vec![item("late", false, Some(now - 1000))];
    items.extend((0..3).map(|_| item("b", false, None)));
    let line = of(&PaneContent::Todo(crate::todopane::test_pane(items))).expect("a line");
    let cells = crate::minpreview::cells(&line, 16, 0);
    let text: String = cells.iter().map(|c| c.c).collect();
    assert!(text.starts_with("1 overdue"), "{text:?} lost the alarm");
    for c in &cells {
        let loud = (c.col as usize) < 2 + "1 overdue".len();
        let want = if loud { t.bell } else { t.text_muted };
        assert_eq!(c.fg, want, "{:?} at {}", c.c, c.col);
    }
    // Only the exact alarm shape is loud: a terminal line is not.
    assert_eq!(alarm("1 overdue"), 9);
    assert_eq!(alarm("12 overdue \u{b7} 3 open"), 10);
    assert_eq!(alarm("overdue"), 0);
    assert_eq!(alarm("3 overdue invoices"), 0);
}

/// A viewer says where in the file you are, since the legend already says
/// which file it is.
#[test]
fn a_viewer_says_where_in_the_file_it_is() {
    let loading = crate::viewpane::ViewPane::open(std::env::temp_dir().join("a.txt"));
    assert_eq!(of(&PaneContent::View(loading)), None, "still loading");
    let mut v = crate::viewpane::ViewPane::open(std::env::temp_dir().join("a.txt"));
    v.state = crate::viewpane::LoadState::Ready {
        format: crate::viewpane::detect::Format::Text,
        loaded: crate::viewpane::load::Loaded {
            text: "one\ntwo\nthree\nfour\n".into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    v.scroll = 2;
    assert_eq!(of(&PaneContent::View(v)).as_deref(), Some("line 3 of 4"));
}

/// Nothing a drawn pane says is ever more than a line, on any state — the
/// strip has one row and a clipped number is a wrong number.
#[test]
fn every_answer_is_one_line() {
    let panes = [
        PaneContent::Todo(crate::todopane::test_pane(vec![item("a", false, None)])),
        PaneContent::Disk(crate::diskpane::DiskPane::new(std::env::temp_dir())),
        PaneContent::Usage(crate::usagepane::UsagePane::default()),
    ];
    for p in panes {
        let line = of(&p).expect("a drawn pane answers");
        assert!(!line.contains('\n'), "{line:?} is two lines");
        assert!(!line.trim().is_empty(), "an empty answer");
    }
}
