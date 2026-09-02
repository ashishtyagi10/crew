use super::*;
use crate::daemon::intent::Repeat;

const NOW: u64 = 1_787_788_800_000;

fn card(id: &str, fire_in: u64, repeat: &str, text: &str) -> IntentCard {
    IntentCard {
        id: id.into(),
        text: text.into(),
        to: String::new(),
        fire_ms: NOW + fire_in,
        repeat: repeat.into(),
        created_ms: NOW,
    }
}

/// Exactly the soonest, whatever order the list arrived in — and nothing else.
#[test]
fn the_line_is_the_soonest_and_only_the_soonest() {
    let cards = [
        card("w2", 5 * 86_400_000, "once", "chase the invoice"),
        card("w3", 30 * 60_000, "every 30m", "check the deploy"),
        card("w1", 2 * 3_600_000, "daily", "brief me"),
    ];
    assert_eq!(line(&cards, NOW), "w3  in 30m  every 30m  check the deploy");
    assert_eq!(line(&cards, NOW).lines().count(), 1);
}

/// A quiet clock is a sentence, not an error.
#[test]
fn nothing_standing_is_the_sentence() {
    assert_eq!(line(&[], NOW), NOTHING);
    assert_eq!(soonest(&[], NOW), NOTHING);
}

/// The channel's answer is the same line, from the live fold.
#[test]
fn the_channel_gets_the_same_line() {
    let it = |id: &str, fire_in: u64| Intent {
        id: id.into(),
        text: "the forecast".into(),
        to: String::new(),
        fire_ms: NOW + fire_in,
        repeat: Repeat::Every { secs: 86_400 },
        created_ms: NOW,
        anchor_ms: None,
    };
    assert_eq!(
        soonest(&[it("w1", 3_600_000), it("w2", 60_000)], NOW),
        "w2  in 1m  daily  the forecast"
    );
}
