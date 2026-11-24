use super::*;

fn day(y: i32, m: u32, d: u32) -> i64 {
    days_from_civil(y, m, d)
}

#[test]
fn civil_days_match_known_epochs() {
    assert_eq!(day(1970, 1, 1), 0);
    assert_eq!(day(2000, 3, 1), 11_017);
    assert_eq!(day(2026, 12, 11) - day(2026, 11, 24), 17);
    assert_eq!(day(2027, 1, 14) - day(2026, 12, 31), 14);
}

#[test]
fn before_within_and_past_the_date() {
    let far = sunset("gpt-5", day(2026, 11, 1)).unwrap();
    assert_eq!(far.days_left, 40);
    assert!(!far.soon());
    assert_eq!(far.badge(), "retires Dec 11");
    // Day 15 is quiet, day 14 speaks.
    assert!(!sunset("gpt-5", day(2026, 11, 26)).unwrap().soon());
    let near = sunset("gpt-5", day(2026, 11, 27)).unwrap();
    assert!(near.soon());
    assert_eq!(
        near.line("gpt-5"),
        "gpt-5 retires Dec 11 \u{2014} gpt-6.1-sol next"
    );
    // On the day calls stop, so it is already retired.
    let on = sunset("gpt-5", day(2026, 12, 11)).unwrap();
    assert_eq!(on.days_left, 0);
    assert_eq!(on.badge(), "retired");
    let past = sunset("gpt-5", day(2027, 2, 1)).unwrap();
    assert!(past.soon());
    assert_eq!(
        past.line("gpt-5"),
        "gpt-5 retired Dec 11 \u{2014} gpt-6.1-sol next"
    );
}

#[test]
fn dated_snapshots_share_their_base_date() {
    let t = day(2026, 11, 24);
    for id in ["gpt-5-2025-08-07", "o3-2025-04-16", "gpt-4o-2024-11-20"] {
        assert!(sunset(id, t).is_some(), "{id}");
    }
    let s = sunset("claude-sonnet-4-5-20250929", t).unwrap();
    assert_eq!((s.date, s.next), ((2026, 11, 30), "claude-sonnet-5-5"));
    assert_eq!(
        sunset("gpt-5-mini-2025-08-07", t).unwrap().next,
        "gpt-6.1-terra"
    );
    assert_eq!(sunset("gpt-4.1", t).unwrap().day(), "Jan 14");
    // Read off the vendor pages 2026-11-24: the minis, Opus 4.5 and 2.5 Flash have dates too.
    assert_eq!(sunset("gpt-4o-mini-2024-07-18", t).unwrap().day(), "Jan 14");
    assert_eq!(
        sunset("claude-opus-4-5", t).unwrap().next,
        "claude-opus-5-5"
    );
    assert!(sunset("gemini-2.5-flash", t).unwrap().routed);
}

#[test]
fn neighbours_of_a_dated_id_are_untouched() {
    let t = day(2026, 11, 24);
    for id in [
        "gpt-5.5",
        "gpt-6.1-sol",
        "gpt-4o-mini-tts",
        "gpt-4.1-nano",
        "o3-mini",
        "gemini-2.5-flash-lite",
        "gpt-5-1",
        "gemini-2.5-pro-preview-tts",
        "claude-sonnet-5-5",
        "openai/gpt-5",
        "",
    ] {
        assert_eq!(sunset(id, t), None, "{id}");
    }
}

#[test]
fn a_rerouted_model_is_replaced_not_retired() {
    let g = sunset("gemini-2.5-pro", day(2026, 11, 24)).unwrap();
    assert_eq!(g.badge(), "replaced Dec 1");
    assert_eq!(
        g.line("gemini-2.5-pro"),
        "gemini-2.5-pro is replaced Dec 1 \u{2014} gemini-3.8-flash next"
    );
    assert_eq!(
        sunset("gemini-2.5-pro", day(2026, 12, 2)).unwrap().badge(),
        "replaced"
    );
}

#[test]
fn today_is_after_this_table_was_written() {
    assert!(today() >= day(2026, 11, 24));
}
