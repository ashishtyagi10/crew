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
        anchor_ms: None,
    }
}

#[test]
fn nothing_standing_says_how_to_set_one() {
    let out = listing(&[], &Default::default(), NOW);
    assert!(out.contains("Nothing standing"), "{out}");
    // The advice is one paragraph, not lines broken by hand to a tile.
    assert_eq!(out.lines().count(), 3, "{out}");
    assert!(
        out.lines()
            .nth(2)
            .unwrap()
            .ends_with("said over a channel."),
        "{out}"
    );
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
        &Default::default(),
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

/// `/watching snooze w1 30m` is the third verb, and its status line says where it landed.
#[test]
fn snooze_from_the_app_says_where_it_landed() {
    let p = std::env::temp_dir().join(format!(
        "crew-watchview-snooze-{}.jsonl",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&p);
    let list = Watchlist::at(&p);
    let it = list
        .add(
            "noon",
            "",
            NOW + HOUR,
            crate::daemon::intent::Repeat::Once,
            NOW,
        )
        .unwrap();
    assert_eq!(snooze(&list, " w1 30m", NOW), "w1 snoozed \u{2014} in 30m");
    assert_eq!(list.live()[0].fire_ms, NOW + 30 * 60_000, "{}", it.id);
    assert_eq!(snooze(&list, "w7 30m", NOW), "crew is not watching for w7");
    assert!(snooze(&list, "w1", NOW).starts_with("usage:"));
    assert!(snooze(&list, "w1 daily", NOW).contains("how long"));
    let _ = std::fs::remove_file(&p);
}

/// A row that has fired says so under itself — how many times, when last, and what it
/// missed — and one that never has says nothing extra.
#[test]
fn a_row_says_what_it_has_already_done() {
    let mut history = std::collections::BTreeMap::new();
    history.insert(
        "w1".to_string(),
        crate::daemon::intenthistory::Fired {
            count: 40,
            last_ms: NOW - 22 * HOUR,
            missed: 3,
        },
    );
    let out = listing(
        &[
            intent(
                "w1",
                "brief me",
                "telegram:42",
                2 * HOUR,
                Repeat::Every { secs: 86_400 },
            ),
            intent("w2", "chase the invoice", "", 5 * 24 * HOUR, Repeat::Once),
        ],
        &history,
        NOW,
    );
    let detail = out.lines().find(|l| l.contains("telegram:42")).unwrap();
    assert_eq!(
        detail,
        "     \u{2192} telegram:42 \u{b7} standing 3d \u{b7} fired 40\u{d7} \u{b7} last 22h ago \u{b7} 3 missed"
    );
    let w2 = out
        .lines()
        .skip_while(|l| !l.starts_with("w2"))
        .nth(1)
        .unwrap();
    assert_eq!(w2, "     standing 3d", "never fired: nothing extra");
}
