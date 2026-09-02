use super::*;

const NOW: u64 = 1_787_788_800_000;

fn rec(ago_s: u64, tool: &str, decision: &str, outcome: &str) -> Record {
    Record {
        ts_ms: NOW - ago_s * 1000,
        tool: tool.into(),
        tier: "read".into(),
        requester: "pane".into(),
        decision: decision.into(),
        outcome: outcome.into(),
        note: String::new(),
    }
}

/// The ask claims only what it names; a sentence that merely contains "tools" is a task.
#[test]
fn the_ask_is_explicit_and_a_trailing_word_narrows_it() {
    assert_eq!(read("tools"), Some(String::new()));
    assert_eq!(read("/tools gmail"), Some("gmail".into()));
    assert_eq!(read("Ledger"), Some(String::new()));
    assert_eq!(read("what have you done?"), Some(String::new()));
    assert_eq!(read("what did you do with gmail?"), Some("gmail".into()));
    assert_eq!(read("What have you run today"), Some("today".into()));
    assert_eq!(read("install the tools for rust"), None);
    assert_eq!(read("what tools do you have"), None);
    assert_eq!(read(""), None);
}

/// Oldest first, newest nearest the thumb; the unusual ones say what was unusual.
#[test]
fn the_answer_is_the_last_few_with_the_newest_last() {
    let mut r = rec(30, "gmail:send", "ask", "denied");
    r.requester = "channel:telegram:42".into();
    let out = answer(
        &[
            rec(3_600, "sys:list_dir", "allow", "ran"),
            rec(60, "sys:run", "allow", "ran"),
            r,
        ],
        "",
        NOW,
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    assert!(lines[0].contains("sys:list_dir"), "{out}");
    assert!(lines[2].contains("gmail:send"), "newest last: {out}");
    assert!(lines[2].contains("\u{2717}"), "{out}");
    assert!(lines[2].contains("denied"), "{out}");
    assert!(lines[2].contains("channel:telegram:42"), "{out}");
    assert!(!lines[1].contains("ran"), "the tick already said it: {out}");
}

/// A screen, not a listing: ten rows, and a count of what is before them.
#[test]
fn more_than_a_screen_is_cut_and_says_by_how_much() {
    let records: Vec<Record> = (0..14)
        .map(|i| rec(100 - i, "sys:run", "allow", "ran"))
        .collect();
    let out = answer(&records, "", NOW);
    assert_eq!(out.lines().count(), ROWS + 1, "{out}");
    assert!(out.starts_with("4 more before these"), "{out}");
}

#[test]
fn a_filter_narrows_and_an_empty_ledger_says_so() {
    let records = [
        rec(10, "sys:run", "allow", "ran"),
        rec(5, "gmail:send", "allow", "ran"),
    ];
    let out = answer(&records, "GMAIL", NOW);
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(out.contains("gmail:send"));
    assert_eq!(
        answer(&records, "slack", NOW),
        "nothing in the ledger matches \"slack\""
    );
    assert_eq!(answer(&[], "", NOW), "I have not run anything yet");
}

/// From the daemon: the ledger it was pointed at, and never while an approval is pending —
/// "tools" then is somebody typing, not asking, and the answer would steal the question.
#[test]
fn the_daemon_answers_from_its_ledger_unless_the_sender_is_mid_approval() {
    let mut rig = crate::daemon::clock::tests::rig("ledgerchat");
    let path = std::env::temp_dir().join(format!("crew-ledgerchat-{}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&path);
    rig.d.set_ledger(&path);
    assert_eq!(
        rig.d.ledger_chat("test:1", "tools", NOW).as_deref(),
        Some("I have not run anything yet")
    );
    crew_plugin::ledger::Ledger::at(&path)
        .append(&rec(30, "weather:forecast", "allow", "ran"))
        .unwrap();
    let out = rig
        .d
        .ledger_chat("test:1", "what have you done", NOW)
        .unwrap();
    assert!(out.contains("weather:forecast"), "{out}");
    assert_eq!(rig.d.ledger_chat("test:1", "book me a flight", NOW), None);
    rig.d.bridge.hold("test:1", "a1");
    assert_eq!(
        rig.d.ledger_chat("test:1", "tools", NOW),
        None,
        "mid-approval"
    );
    let _ = std::fs::remove_file(&path);
}
