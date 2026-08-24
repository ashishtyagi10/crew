use super::*;
use crew_plugin::approval::Requester;
use crew_plugin::tier::Tier;

#[test]
fn a_line_shows_the_decision_the_tier_and_who_asked() {
    let r = Record::decided(
        "gmail:send",
        Tier::Irreversible,
        &Requester::Channel("telegram:me".into()),
        "ask",
        "a1",
    )
    .with_outcome("granted");
    let out = line(&r);
    for needle in [
        "ask",
        "irreversible",
        "channel:telegram:me",
        "gmail:send",
        "granted",
        "a1",
    ] {
        assert!(out.contains(needle), "{needle:?} missing from {out:?}");
    }
}

/// A record whose outcome is not yet known must not render a dangling arrow.
#[test]
fn an_undecided_record_renders_without_an_outcome() {
    let r = Record::decided(
        "sys:run",
        Tier::Irreversible,
        &Requester::LocalPane,
        "allow",
        "",
    );
    let out = line(&r);
    assert!(
        !out.contains('\u{2192}'),
        "no arrow with nothing after it: {out:?}"
    );
    assert!(out.contains("sys:run"));
}

#[test]
fn the_stamp_wraps_at_a_day_and_pads() {
    assert_eq!(stamp(0), "00:00:00");
    assert_eq!(stamp(61_000), "00:01:01");
    assert_eq!(
        stamp(86_400_000),
        "00:00:00",
        "a day later reads as midnight again"
    );
    assert_eq!(stamp(3_661_000), "01:01:01");
}
