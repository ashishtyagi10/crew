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
    append_at(&base, "crew", "starting with coder…");
    append_at(&base, "planner", "   ");
    let text = std::fs::read_to_string(live(&base)).unwrap();
    assert!(text.contains("user: fix the flaky test"));
    assert!(text.contains("coder → user: done"));
    assert!(!text.contains("starting with"), "crew voice skipped");
    assert_eq!(text.lines().count(), 2, "blank text skipped");
}

#[test]
fn append_caps_the_live_log_by_dropping_the_oldest_half() {
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
