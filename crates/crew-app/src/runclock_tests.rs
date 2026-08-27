use super::*;

fn at(secs: u64) -> Option<String> {
    label(Duration::from_secs(secs))
}

/// Every command is briefly a running command; a clock on every `ls` is
/// chrome rather than information.
#[test]
fn nothing_is_drawn_for_a_command_that_just_started() {
    assert_eq!(at(0), None);
    assert_eq!(at(MIN_SECS - 1), None);
    assert!(at(MIN_SECS).is_some());
}

#[test]
fn the_scale_changes_with_the_wait() {
    assert_eq!(at(9).as_deref(), Some("9s"));
    assert_eq!(at(59).as_deref(), Some("59s"));
    assert_eq!(at(60).as_deref(), Some("1m00"));
    assert_eq!(at(134).as_deref(), Some("2m14"));
    assert_eq!(at(3599).as_deref(), Some("59m59"));
    assert_eq!(at(3600).as_deref(), Some("1h00"));
    assert_eq!(at(7 * 3600 + 25 * 60).as_deref(), Some("7h25"));
}

/// The border is shared with the legend, the git badge and the status
/// glyphs — the clock has to stay small at every scale, including absurd ones.
#[test]
fn the_label_never_outgrows_its_slot() {
    for secs in [5u64, 59, 60, 599, 3599, 3600, 86_400, 400_000] {
        let l = at(secs).unwrap();
        assert!(l.chars().count() <= 5, "{secs}s → {l}");
    }
}

/// Minutes and hours are zero-padded so the clock does not jitter in width
/// as it counts — a number that changes width every ten seconds reads as
/// something moving rather than something elapsing.
#[test]
fn the_minutes_and_hours_forms_hold_their_width() {
    assert_eq!(
        at(65).unwrap().chars().count(),
        at(119).unwrap().chars().count()
    );
    assert_eq!(
        at(3605).unwrap().chars().count(),
        at(7000).unwrap().chars().count()
    );
}

/// On the card: the clock rides the top border, and an idle pane's border is
/// exactly what it was.
#[test]
fn the_card_carries_the_clock_only_while_something_is_running() {
    let _g = crate::app::theme_test_guard();
    let bar = |elapsed| crate::panecard::Bar {
        index: Some(1),
        title: "zsh",
        focused: false,
        scroll: 0,
        total: 0,
        activity: false,
        bell: false,
        broadcast: false,
        min_btn: false,
        assemble_t: 1.0,
        focus_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        elapsed,
        err_rows: &[],
        unread: 0,
        doc: false,
    };
    let row0 = |elapsed: Option<String>| -> String {
        let mut cells = crate::panecard::pane_card(60, 8, &bar(elapsed));
        cells.retain(|c| c.row == 0);
        cells.sort_by_key(|c| c.col);
        cells.iter().map(|c| c.c).collect()
    };
    let idle = row0(None);
    let busy = row0(Some("2m14".into()));
    assert!(busy.contains("2m14"), "{busy}");
    assert!(!idle.contains("2m14"));
    assert_eq!(idle.chars().count(), busy.chars().count(), "the row moved");
    assert!(busy.contains("1 zsh"), "the legend was pushed off: {busy}");
}
