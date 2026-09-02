use super::*;
use crate::daemon::intent::Repeat;

/// A fixed 'now': 2026-08-31T00:00:00Z.
const NOW: u64 = 1_787_788_800_000;
const HOUR: u64 = 3_600_000;

fn intent(id: &str, text: &str, to: &str, fire_in: u64, repeat: Repeat) -> Intent {
    Intent {
        id: id.into(),
        text: text.into(),
        to: to.into(),
        fire_ms: NOW + fire_in,
        repeat,
        created_ms: NOW - 3 * 24 * HOUR,
    }
}

#[test]
fn nothing_standing_says_how_to_set_one() {
    let out = listing(&[], NOW);
    assert!(out.contains("Nothing standing"), "{out}");
    assert!(out.contains("crew daemon at"), "{out}");
    assert!(
        !out.contains("standing \u{b7}"),
        "no count row for no rows: {out}"
    );
}

#[test]
fn a_row_says_when_how_often_and_what_and_the_detail_says_where_and_since() {
    let out = listing(
        &[
            intent(
                "w1",
                "brief me on the calendar",
                "telegram:42",
                2 * HOUR + 14 * 60_000,
                Repeat::Every { secs: 86_400 },
            ),
            intent("w2", "chase the invoice", "", 5 * 24 * HOUR, Repeat::Once),
        ],
        NOW,
    );
    assert!(out.contains("2 standing"), "{out}");
    assert!(out.contains("/watching cancel <id>"), "{out}");
    let w1 = out.lines().find(|l| l.starts_with("w1")).expect("w1 row");
    assert_eq!(w1, "w1   in 2h     daily       brief me on the calendar");
    let detail = out
        .lines()
        .find(|l| l.contains("telegram:42"))
        .expect("where it goes");
    assert_eq!(detail, "     \u{2192} telegram:42 \u{b7} standing 3d");
    let w2 = out.lines().find(|l| l.starts_with("w2")).expect("w2 row");
    assert!(
        w2.contains("in 5d") && w2.contains("once") && w2.ends_with("chase the invoice"),
        "{w2}"
    );
    // No channel: the detail says only how long it has stood.
    let after_w2 = out
        .lines()
        .skip_while(|l| !l.starts_with("w2"))
        .nth(1)
        .unwrap_or("");
    assert_eq!(after_w2, "     standing 3d");
}

/// A cancel from the app is an append to the same log the daemon folds.
#[test]
fn cancel_writes_the_log_the_clock_reads() {
    let p = std::env::temp_dir().join(format!("crew-watchview-{}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&p);
    let list = Watchlist::at(&p);
    let it = list
        .add("the forecast", "", NOW + HOUR, Repeat::Once, NOW)
        .unwrap();
    assert_eq!(list.live().len(), 1);
    assert_eq!(cancel(&list, "w9", NOW), "crew is not watching for w9");
    assert_eq!(cancel(&list, "", NOW), "usage: /watching cancel <id>");
    assert_eq!(list.live().len(), 1, "a miss cancels nothing");
    assert_eq!(cancel(&list, &format!(" {} ", it.id), NOW), "w1 cancelled");
    assert!(
        Watchlist::at(&p).live().is_empty(),
        "gone for whoever reads the file next"
    );
    let _ = std::fs::remove_file(&p);
}
