use super::*;

fn snap(sessions: Vec<Card>) -> Snapshot {
    Snapshot {
        version: "9.9.9".into(),
        uptime_s: 3_725,
        sessions,
    }
}

fn card(id: &str, alive: bool) -> Card {
    Card {
        id: id.into(),
        label: "crew".into(),
        cwd: None,
        alive,
    }
}

#[test]
fn status_reports_the_version_uptime_and_session_count() {
    let out = respond("status", &snap(vec![card("s1", true)]));
    assert!(out.contains("9.9.9"), "{out}");
    assert!(out.contains("1h2m"), "uptime is readable: {out}");
    assert!(out.contains("1 session"), "{out}");
}

#[test]
fn sessions_lists_each_one_with_its_state() {
    let out = respond("sessions", &snap(vec![card("s1", true), card("s2", false)]));
    assert!(out.contains("s1  running"), "{out}");
    assert!(out.contains("s2  dead"), "{out}");
    assert_eq!(respond("sessions", &snap(vec![])), "no sessions");
}

/// A message that goes unanswered is indistinguishable from a crew that is down — the one thing
/// a remote channel must never look like.
#[test]
fn an_unknown_message_still_gets_an_answer_that_says_what_is_possible() {
    let out = respond("book me a flight to Berlin", &snap(vec![]));
    assert!(!out.is_empty());
    for w in KNOWN {
        assert!(out.contains(w), "the fallback lists {w}: {out}");
    }
}

/// Telegram users type `/status`; people type `status`. Both are the same question.
#[test]
fn slash_commands_and_plain_words_are_the_same_question() {
    let s = snap(vec![]);
    assert_eq!(respond("/status", &s), respond("status", &s));
    assert_eq!(respond("/help", &s), respond("help", &s));
    assert_eq!(respond("/start", &s), respond("help", &s));
    assert_eq!(
        respond("STATUS", &s),
        respond("status", &s),
        "case does not matter"
    );
    assert_eq!(
        respond("  status  ", &s),
        respond("status", &s),
        "nor does surrounding space"
    );
}

/// Only the first word decides — "status of my flight" is still the status command, and that is
/// a deliberate simplicity, not an accident to be surprised by later.
#[test]
fn the_first_word_decides() {
    let s = snap(vec![]);
    assert_eq!(respond("status please", &s), respond("status", &s));
}

/// The help text and the fallback must not drift apart.
#[test]
fn help_names_every_command_the_fallback_advertises() {
    let out = respond("help", &snap(vec![]));
    for w in KNOWN {
        assert!(out.contains(w), "help omits {w}: {out}");
    }
}

#[test]
fn uptime_reads_naturally_at_every_scale() {
    assert_eq!(human_uptime(9), "9s");
    assert_eq!(human_uptime(90), "1m");
    assert_eq!(human_uptime(3_725), "1h2m");
    assert_eq!(human_uptime(90_000), "1d1h");
}
