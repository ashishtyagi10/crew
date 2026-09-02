//! The line between "an alarm" and "a task for an agent" is the whole risk here: a parse that
//! claims too much turns "book me a flight tomorrow" into a reminder and never does the work.
use super::*;

fn now_ms() -> u64 {
    crate::chattime::unix_now_ms()
}

fn registered(said: &str) -> (String, Repeat) {
    match read(said, now_ms()) {
        Some(Ok(Ask::Register { text, repeat, .. })) => (text, repeat),
        other => panic!("expected a registration from {said:?}, got {other:?}"),
    }
}

#[test]
fn a_reminder_keeps_the_errand_and_takes_the_time() {
    let (text, repeat) = registered("remind me tomorrow 9am to call the bank");
    assert_eq!(text, "call the bank");
    assert_eq!(repeat, Repeat::Once);
}

#[test]
fn the_word_me_is_optional_and_so_is_the_word_to() {
    assert_eq!(
        registered("remind tomorrow 9am call the bank").0,
        "call the bank"
    );
    assert_eq!(
        registered("remind me tomorrow 9am call the bank").0,
        "call the bank"
    );
}

#[test]
fn a_cadence_said_out_loud_becomes_a_repeat_and_leaves_the_errand_alone() {
    let (text, repeat) = registered("remind me every day tomorrow 7am to brief me");
    assert_eq!(
        text, "brief me",
        "the cadence words are not part of the task"
    );
    assert_eq!(repeat, Repeat::Every { secs: 86_400 });
    assert_eq!(
        registered("remind me weekly tomorrow 9am to water the plants").1,
        Repeat::Every { secs: 604_800 }
    );
    assert_eq!(
        registered("remind me every 30m tomorrow 9am to stand up").1,
        Repeat::Every { secs: 1_800 }
    );
}

#[test]
fn anything_that_is_not_a_watch_command_is_left_for_an_agent() {
    // This is the important one. Each of these has a time in it and none of them is an alarm.
    for task in [
        "book me a flight tomorrow",
        "what is on my calendar tomorrow 9am",
        "every day I lose an hour to this, fix it",
        "status",
        "",
    ] {
        assert!(read(task, now_ms()).is_none(), "{task:?} was claimed");
    }
}

#[test]
fn a_bare_cancel_is_an_answer_to_a_question_not_a_command() {
    // While an agent is blocked on an approval, "cancel" means no. Reading it as a watch
    // command would answer the wrong question and leave the agent hanging.
    assert!(read("cancel", now_ms()).is_none());
    assert!(read("cancel that", now_ms()).is_none());
    assert_eq!(
        read("cancel w3", now_ms()),
        Some(Ok(Ask::Cancel("w3".into())))
    );
}

#[test]
fn asking_what_is_watched_is_a_command() {
    assert_eq!(read("watching", now_ms()), Some(Ok(Ask::List)));
    assert_eq!(read("reminders", now_ms()), Some(Ok(Ask::List)));
}

#[test]
fn a_reminder_with_no_time_in_it_asks_when_rather_than_guessing() {
    let e = match read("remind me to call the bank", now_ms()) {
        Some(Err(e)) => e,
        other => panic!("expected a question back, got {other:?}"),
    };
    assert!(e.contains("when?"), "{e}");
}

#[test]
fn a_reminder_for_a_time_that_has_passed_says_so() {
    let fire = match read("remind me tomorrow 9am to call the bank", now_ms()) {
        Some(Ok(Ask::Register { fire_ms, .. })) => fire_ms,
        other => panic!("expected a registration, got {other:?}"),
    };
    let e = match read("remind me tomorrow 9am to call the bank", fire + 1) {
        Some(Err(e)) => e,
        other => panic!("expected a refusal, got {other:?}"),
    };
    assert!(e.contains("already passed"), "{e}");
}

/// "snooze w1 30m" is a watch command; a bare "snooze" is not — somebody may be telling an
/// agent to snooze something — and "snooze w1" with no duration IS one, answered with what
/// would have worked rather than handed to an agent as a task.
#[test]
fn a_snooze_needs_an_id_and_says_so_without_a_duration() {
    assert!(read("snooze", now_ms()).is_none());
    assert!(read("snooze the alarm", now_ms()).is_none());
    assert_eq!(
        read("snooze w1 30m", now_ms()),
        Some(Ok(Ask::Snooze {
            id: "w1".into(),
            delay_ms: 1_800_000
        }))
    );
    assert_eq!(
        read("/snooze W2 2h", now_ms()),
        Some(Ok(Ask::Snooze {
            id: "w2".into(),
            delay_ms: 7_200_000
        }))
    );
    let e = read("snooze w1", now_ms()).unwrap().unwrap_err();
    assert!(e.contains("30m"), "{e}");
    let e = read("snooze w1 daily", now_ms()).unwrap().unwrap_err();
    assert!(e.contains("how long"), "a cadence is not a duration: {e}");
}

/// "what's next" is a question for the clock; "what's next on the roadmap" is a task.
#[test]
fn whats_next_is_a_command_and_only_on_its_own() {
    assert_eq!(read("next", now_ms()), Some(Ok(Ask::Next)));
    assert_eq!(read("What's next?", now_ms()), Some(Ok(Ask::Next)));
    assert_eq!(read("whats next", now_ms()), Some(Ok(Ask::Next)));
    assert_eq!(read("what is next", now_ms()), Some(Ok(Ask::Next)));
    assert!(read("what's next on the roadmap", now_ms()).is_none());
    assert!(read("what", now_ms()).is_none());
}
