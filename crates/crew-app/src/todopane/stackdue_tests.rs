//! On a stacked row the due gives up words before the chips give up room.
use crate::todopane::duetext::fit_label;
use crate::todopane::item::TodoItem;
use crate::todopane::render::cells;

#[test]
fn the_due_label_sheds_its_word_then_its_time() {
    let now = crate::chattime::unix_now_ms();
    let due = now + 60 * 60 * 1000;
    let full = fit_label(due, true, now, 99);
    assert!(
        full.starts_with("today ") || full.starts_with("tomorrow "),
        "{full}"
    );
    let dated = fit_label(due, true, now, full.len() - 1);
    assert!(!dated.starts_with("to") && dated.contains(':'), "{dated}");
    let bare = fit_label(due, true, now, dated.len() - 1);
    assert!(!bare.contains(':'), "{bare}");
}

/// The narrow row from the survey: `#priya @crew` and a due timed today, on
/// a pane too narrow for them all whole — the chips stay, the due shortens.
#[test]
fn a_narrow_dated_row_keeps_its_chips() {
    let _g = crate::app::theme_test_guard();
    let now = crate::chattime::unix_now_ms();
    let it = TodoItem {
        id: 1,
        title: "ship the release notes".into(),
        project: Some("crew".into()),
        assignee: Some("priya".into()),
        due_ms: Some(now + 60 * 60 * 1000),
        due_has_time: true,
        ..Default::default()
    };
    let mut p = crate::todopane::test_pane(vec![it]);
    p.grouped = false;
    let all: String = cells(&p, 34, 12).iter().map(|c| c.c).collect();
    assert!(all.contains("#priya") && all.contains("@crew"), "{all}");
}
