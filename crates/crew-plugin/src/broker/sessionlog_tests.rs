use super::*;

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crew-seslog-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn append_logs_conversation_but_skips_system_noise() {
    let base = scratch("append");
    append_at(&base, "user", "fix the flaky test");
    append_at(&base, "coder → user", "done — it raced on the clock");
    append_at(&base, "agent smith", "starting with coder…");
    append_at(&base, "planner", "   ");
    let text = std::fs::read_to_string(live(&base)).unwrap();
    assert!(text.contains("user: fix the flaky test"));
    assert!(text.contains("coder → user: done"));
    assert!(!text.contains("starting with"), "agent smith voice skipped");
    assert_eq!(text.lines().count(), 2, "blank text skipped");
}

#[test]
fn append_caps_the_live_log_by_dropping_the_oldest_half() {
    // Keyless, deliberately: `append_at`'s fold reaches for the LIVE
    // summarizer, so on a machine with a real provider key this test made
    // real network calls per fold and asserted against whatever the model
    // wrote — "oldest dropped" failed whenever a summary happened to echo
    // "reply number 0". The guard pins the clipping branch, which is the
    // behavior these assertions describe; the summarized branch is covered
    // by `compact_tests` with an injected call.
    let _g = crate::broker::testenv::no_provider();
    let base = scratch("cap");
    for i in 0..2000 {
        append_at(
            &base,
            "coder",
            &format!("reply number {i} {}", "x".repeat(40)),
        );
    }
    let text = std::fs::read_to_string(live(&base)).unwrap();
    assert!(text.len() <= LOG_CAP, "capped: {} bytes", text.len());
    assert!(text.contains("reply number 1999"), "newest survives");
    assert!(!text.contains("reply number 0 "), "oldest dropped");
}

#[test]
fn rotate_promotes_live_to_last_and_starts_fresh() {
    let base = scratch("rotate");
    append_at(&base, "user", "session one");
    rotate_at(&base);
    assert!(!live(&base).exists(), "live log starts fresh");
    let l = std::fs::read_to_string(last(&base)).unwrap();
    assert!(l.contains("session one"));
    // a second rotation with no new live log keeps the last session
    rotate_at(&base);
    assert!(
        last(&base).exists(),
        "empty session doesn't wipe the resumable one"
    );
}

#[test]
fn tail_reads_the_last_session_bounded() {
    let base = scratch("tail");
    assert_eq!(tail_at(&base), None, "nothing to resume yet");
    for i in 0..200 {
        append_at(&base, "coder", &format!("line {i} {}", "y".repeat(30)));
    }
    rotate_at(&base);
    let t = tail_at(&base).unwrap();
    assert!(t.len() <= RESUME_CAP + 40);
    assert!(t.contains("line 199"), "keeps the newest lines");
}

#[test]
fn with_resume_frames_context_before_the_task() {
    let p = with_resume("coder: it was the cache", "now fix the docs");
    let ctx = p.find("it was the cache").unwrap();
    let task = p.find("now fix the docs").unwrap();
    assert!(ctx < task, "context precedes the task");
    assert!(
        p.to_uppercase().contains("PREVIOUS SESSION"),
        "labeled as restored context: {p}"
    );
}

/// The log has rotated into a resumable file since long before this and said
/// so nowhere: a user opening a pane in yesterday's project had their
/// conversation held for them with no way to find out.
#[test]
fn a_previous_session_offers_itself_and_names_the_construct() {
    let base = scratch("offer");
    append_at(&base, "user", "fix the flaky test");
    append_at(&base, "coder → user", "done — it raced on the clock");
    rotate_at(&base); // the run ends; the live log becomes resumable

    let note = resume_offer_at(&base).expect("a session was there to offer");
    assert!(note.contains("2 messages"), "{note}");
    // `/resume` retired: the offer teaches the plain-language ask instead.
    assert!(
        note.contains("pick up where we left off"),
        "an offer must teach the phrasing: {note}"
    );
    assert!(!note.contains("/resume"), "{note}");
}

#[test]
fn one_message_is_singular() {
    let base = scratch("offer-one");
    append_at(&base, "user", "just the one");
    rotate_at(&base);
    let note = resume_offer_at(&base).unwrap();
    assert!(note.contains("1 message from"), "{note}");
}

/// A first run in a fresh project must say nothing. An offer of nothing is
/// noise on the one screen a first run is guaranteed to see.
#[test]
fn a_first_run_offers_nothing() {
    let base = scratch("offer-none");
    assert_eq!(resume_offer_at(&base), None, "no log at all");
    rotate_at(&base);
    assert_eq!(resume_offer_at(&base), None, "rotated an absent log");
    // …and a session that logged nothing is the same as no session.
    append_at(&base, "agent smith", "starting with coder…");
    rotate_at(&base);
    assert_eq!(resume_offer_at(&base), None, "only system voice was logged");
}
