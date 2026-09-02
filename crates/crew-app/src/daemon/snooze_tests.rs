use super::*;

const DAY: u64 = 86_400_000;

fn list(tag: &str) -> Watchlist {
    let p = std::env::temp_dir().join(format!("crew-snooze-{}-{tag}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&p);
    Watchlist::at(p)
}

/// The delay is a duration, not a cadence: `daily` answers "how often", not "for how long".
#[test]
fn a_delay_is_a_duration_the_cadence_grammar_reads() {
    assert_eq!(delay_ms("30m"), Some(1_800_000));
    assert_eq!(delay_ms("2h"), Some(7_200_000));
    assert_eq!(delay_ms("1d"), Some(DAY));
    assert_eq!(delay_ms("once"), None);
    assert_eq!(delay_ms("daily"), None, "a cadence, not a duration");
    assert_eq!(delay_ms("soon"), None);
}

/// A snoozed one-shot moves, and only it moves.
#[test]
fn a_snooze_moves_the_next_firing_and_nothing_else() {
    let w = list("once");
    let a = w.add("noon", "telegram:42", DAY, Repeat::Once, 0).unwrap();
    let b = w.add("dusk", "", 2 * DAY, Repeat::Once, 0).unwrap();
    assert_eq!(w.snooze(&a.id, 3_600_000, 1_000).unwrap(), Some(3_601_000));
    let live = w.live();
    assert_eq!(
        live[0].id, a.id,
        "soonest first — the snooze landed before b"
    );
    assert_eq!(live[0].fire_ms, 3_601_000);
    assert_eq!(live[1].fire_ms, 2 * DAY, "{}: untouched", b.id);
    assert_eq!(
        w.snooze("w9", 1, 1_000).unwrap(),
        None,
        "nothing by that id"
    );
}

/// The daily briefing snoozed half an hour fires late TODAY and on time tomorrow — the
/// cadence is anchored, or every later firing would inherit the half hour.
#[test]
fn a_snoozed_repeat_keeps_its_cadence() {
    let w = list("repeat");
    let it = w
        .add(
            "briefing",
            "telegram:42",
            7 * 3_600_000,
            Repeat::Every { secs: 86_400 },
            0,
        )
        .unwrap();
    // 07:00, snoozed at 07:01 for 30 minutes → 07:31.
    let at = 7 * 3_600_000 + 60_000;
    assert_eq!(
        w.snooze(&it.id, 1_800_000, at).unwrap(),
        Some(at + 1_800_000)
    );
    let live = w.live();
    assert_eq!(live[0].fire_ms, at + 1_800_000);
    assert_eq!(
        live[0].anchor_ms,
        Some(7 * 3_600_000),
        "the cadence remembers 07:00"
    );
    // It fires at 07:31, and the next one is tomorrow 07:00 — not 07:31.
    w.record_fire(&live[0], at + 1_800_000).unwrap();
    let live = w.live();
    assert_eq!(live[0].fire_ms, DAY + 7 * 3_600_000);
    assert_eq!(live[0].anchor_ms, None, "back on its own cadence");
    // A second snooze on a snoozed intent still counts from the original anchor.
    w.snooze(&it.id, 60_000, DAY + 7 * 3_600_000 + 1).unwrap();
    w.snooze(&it.id, 60_000, DAY + 7 * 3_600_000 + 2).unwrap();
    assert_eq!(w.live()[0].anchor_ms, Some(DAY + 7 * 3_600_000));
}

#[test]
fn the_sentence_says_where_it_landed_or_why_not() {
    assert_eq!(
        said("w1", Ok(Some(3_600_000)), 0),
        "w1 snoozed \u{2014} in 1h"
    );
    assert_eq!(said("w9", Ok(None), 0), "crew is not watching for w9");
    assert!(said("w1", Err(std::io::Error::other("disk")), 0).contains("disk"));
}
