use super::*;

#[test]
fn the_badge_caps_rather_than_growing_a_fourth_digit() {
    assert_eq!(badge(0), None);
    assert_eq!(badge(1).as_deref(), Some("1"));
    assert_eq!(badge(99).as_deref(), Some("99"));
    assert_eq!(badge(100).as_deref(), Some("99+"));
    assert_eq!(badge(4000).as_deref(), Some("99+"));
}

/// On the card: the count rides the top border beside the activity dot, and a
/// pane with nothing new draws no number at all.
#[test]
fn the_card_shows_the_count_and_only_when_there_is_one() {
    let _g = crate::app::theme_test_guard();
    let bar = |unread| crate::panecard::Bar {
        index: Some(2),
        title: "sh",
        focused: false,
        scroll: 0,
        total: 0,
        activity: true,
        bell: false,
        broadcast: false,
        min_btn: false,
        assemble_t: 1.0,
        focus_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        elapsed: None,
        pinned: false,
        at_cmd: None,
        fail_rows: &[],
        cmd_rows: &[],
        err_rows: &[],
        unread,
        doc: false,
    };
    let row0 = |unread| -> String {
        let mut cells = crate::panecard::pane_card(60, 8, &bar(unread));
        cells.retain(|c| c.row == 0);
        cells.sort_by_key(|c| c.col);
        cells.iter().map(|c| c.c).collect()
    };
    assert!(row0(12).contains("12"), "{}", row0(12));
    assert!(!row0(0).contains("12"));
    assert!(
        row0(0).contains('\u{25cf}'),
        "the activity dot went missing"
    );
}

/// Watching is reading: a pane you are looking at, sitting at its live
/// bottom, must never accumulate a count of output that landed in front of
/// your eyes.
#[test]
fn the_pane_you_are_reading_counts_nothing_as_new() {
    // 100 lines read, 12 arrived while you watched.
    let stale = 100;
    let total = 112;
    // The old guard advanced the mark only when `count == 0`, which is only
    // true when the mark is ALREADY at the tail — so it never fired.
    assert_eq!(count(total, stale), 12, "the guard's own condition");
    // Focused, at the live bottom: the mark follows the tail and nothing is
    // new any more.
    let read = follow_tail(stale, true, true, total);
    assert_eq!(read, total);
    assert_eq!(count(total, read), 0);
}

/// The two cases the count is *for* keep their mark.
#[test]
fn a_pane_you_are_not_reading_keeps_its_boundary() {
    // Not focused: this is the case the module exists for.
    assert_eq!(follow_tail(100, false, true, 112), 100);
    // Focused but scrolled back: you are catching up, and the rule is what
    // you are catching up to.
    assert_eq!(follow_tail(100, true, false, 112), 100);
    // …and the count it keeps is the one the border shows.
    assert_eq!(count(112, follow_tail(100, true, false, 112)), 12);
}
