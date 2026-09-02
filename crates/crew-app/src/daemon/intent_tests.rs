use super::*;

const MIN: u64 = 60 * 1000;
const HOUR: u64 = 60 * MIN;

fn intent(fire_ms: u64, repeat: Repeat) -> Intent {
    Intent {
        id: "w1".into(),
        text: "the forecast".into(),
        to: "telegram:42".into(),
        fire_ms,
        repeat,
        created_ms: 0,
        anchor_ms: None,
    }
}

#[test]
fn a_cadence_is_read_the_way_it_is_typed() {
    assert_eq!(Repeat::parse("daily"), Some(Repeat::Every { secs: 86_400 }));
    assert_eq!(
        Repeat::parse("every day"),
        Some(Repeat::Every { secs: 86_400 })
    );
    assert_eq!(
        Repeat::parse("weekly"),
        Some(Repeat::Every { secs: 604_800 })
    );
    assert_eq!(Repeat::parse("hourly"), Some(Repeat::Every { secs: 3_600 }));
    assert_eq!(
        Repeat::parse("every 30m"),
        Some(Repeat::Every { secs: 1_800 })
    );
    assert_eq!(
        Repeat::parse("every 2h"),
        Some(Repeat::Every { secs: 7_200 })
    );
    assert_eq!(Repeat::parse("ONCE"), Some(Repeat::Once));
}

#[test]
fn an_unrecognised_cadence_is_none_and_never_once() {
    // "once" is a promise about how many times this runs. Reading "every fortnight" as it
    // would silently drop every firing after the first.
    for bad in ["every fortnight", "sometimes", "every 0m", "every m", "30"] {
        assert_eq!(Repeat::parse(bad), None, "{bad} parsed");
    }
}

#[test]
fn a_cadence_reads_back_as_the_word_it_came_from() {
    assert_eq!(Repeat::Once.label(), "once");
    assert_eq!(Repeat::Every { secs: 3_600 }.label(), "hourly");
    assert_eq!(Repeat::Every { secs: 86_400 }.label(), "daily");
    assert_eq!(Repeat::Every { secs: 604_800 }.label(), "weekly");
    assert_eq!(Repeat::Every { secs: 1_800 }.label(), "every 30m");
    assert_eq!(Repeat::Every { secs: 7_200 }.label(), "every 2h");
    assert_eq!(Repeat::Every { secs: 172_800 }.label(), "every 2d");
    assert_eq!(Repeat::Every { secs: 90 }.label(), "every 90s");
}

#[test]
fn due_is_at_the_moment_not_after_it() {
    let it = intent(1_000, Repeat::Once);
    assert!(!it.due(999));
    assert!(it.due(1_000));
    assert!(it.due(1_001));
}

#[test]
fn a_one_shot_has_no_next_firing() {
    assert_eq!(intent(1_000, Repeat::Once).advance(1_000), None);
}

#[test]
fn a_repeat_fired_on_time_lands_exactly_one_period_later() {
    let it = intent(10 * HOUR, Repeat::Every { secs: 3_600 });
    assert_eq!(
        it.advance(10 * HOUR),
        Some(Rolled {
            next: 11 * HOUR,
            skipped: 0
        })
    );
}

#[test]
fn a_repeat_missed_for_a_week_rolls_forward_and_counts_what_it_skipped() {
    // Daily, due at t=0, and nothing looked for seven days: the answer is ONE firing tomorrow,
    // not seven this morning.
    let day = 86_400_000;
    let it = intent(0, Repeat::Every { secs: 86_400 });
    let rolled = it.advance(7 * day).expect("daily repeats");
    assert_eq!(
        rolled.next,
        8 * day,
        "the next firing is the first future one"
    );
    assert_eq!(rolled.skipped, 7, "seven occurrences fell in the gap");
}

#[test]
fn the_next_firing_is_always_in_the_future_of_the_one_that_produced_it() {
    // Exactly on a period boundary is the case that a `<` instead of a `<=` gets wrong: the
    // rolled time would equal now, and the intent would fire again on the very next poll.
    let it = intent(0, Repeat::Every { secs: 3_600 });
    let rolled = it.advance(2 * HOUR).expect("hourly repeats");
    assert_eq!(rolled.next, 3 * HOUR);
    assert!(rolled.next > 2 * HOUR);
}

#[test]
fn a_firing_within_the_grace_says_nothing_about_being_late() {
    let it = intent(0, Repeat::Once);
    assert_eq!(it.late_note(0), None);
    assert_eq!(it.late_note(GRACE_MS), None, "exactly the grace is on time");
}

#[test]
fn a_firing_that_waited_for_the_machine_to_wake_says_how_long() {
    let it = intent(0, Repeat::Once);
    let note = it.late_note(4 * HOUR).expect("four hours late");
    assert!(note.contains("4h ago"), "{note}");
    assert!(note.contains("crew was not running"), "{note}");
}

#[test]
fn a_clock_that_ran_backwards_is_not_a_late_firing() {
    // now < fire_ms happens on a clock correction; an unchecked subtraction here would
    // underflow into "due 584942417h ago".
    assert_eq!(intent(10 * HOUR, Repeat::Once).late_note(HOUR), None);
}

#[test]
fn a_duration_is_spelled_in_the_coarsest_unit_that_still_says_something() {
    assert_eq!(spell(0), "0s");
    assert_eq!(spell(59), "59s");
    assert_eq!(spell(60), "1m");
    assert_eq!(spell(3_599), "59m");
    assert_eq!(spell(3_600), "1h");
    assert_eq!(spell(86_399), "23h");
    assert_eq!(spell(86_400), "1d");
    assert_eq!(spell(200_000), "2d");
}

#[test]
fn a_past_due_row_reads_as_now_rather_than_as_a_future_one() {
    assert_eq!(until(0, 10_000), "now");
    assert_eq!(until(10_000, 10_000), "now");
    assert_eq!(until(10_000 + 30 * MIN, 10_000), "in 30m");
    assert_eq!(until(10_000 + 2 * 86_400_000, 10_000), "in 2d");
}

#[test]
fn an_intent_survives_the_round_trip_through_json() {
    // The log is the storage, so a field that does not serialize is a field that is lost on
    // the restart the whole feature exists for.
    let it = intent(1_725_000_000_000, Repeat::Every { secs: 86_400 });
    let back: Intent = serde_json::from_str(&serde_json::to_string(&it).unwrap()).unwrap();
    assert_eq!(back, it);
}
