//! `-` past an overdue due is no news: any bump re-armed the toast, so an
//! overdue row moved further back toasted again a minute later.
use super::super::item::TodoItem;
use super::super::{store, TodoPane};

const DAY: u64 = 86_400_000;

fn due_in(days: i64) -> TodoItem {
    let now = crate::chattime::unix_now_ms() as i64;
    TodoItem {
        id: 1,
        title: "pay rent".into(),
        due_ms: Some((now + days * DAY as i64) as u64),
        created_ms: 1,
        ..Default::default()
    }
}

#[test]
fn a_due_moved_into_the_past_stays_told_and_one_moved_ahead_toasts_again() {
    let _g = store::test_guard(vec![TodoItem {
        notified: true,
        ..due_in(-2)
    }]);
    let mut p = TodoPane::new();
    p.bump_due_at(0, false);
    assert!(
        store::snapshot()[0].notified,
        "three days late: no second toast"
    );
    for _ in 0..4 {
        p.bump_due_at(0, true);
    }
    assert!(
        !store::snapshot()[0].notified,
        "a day ahead: it toasts when it comes"
    );
}
