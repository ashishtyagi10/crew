use super::*;

#[test]
fn record_returns_a_message_naming_the_command_and_pane() {
    let mut n = Notifier::default();
    let msg = n
        .record(
            NotifyKind::AgentDone,
            "crew".into(),
            "claude".into(),
            Instant::now(),
        )
        .expect("a fresh event surfaces");
    assert!(msg.contains("claude"));
    assert!(msg.contains("crew"));
    assert_eq!(n.len(), 1);
}

#[test]
fn identical_event_within_cooldown_is_throttled() {
    let mut n = Notifier::default();
    let t0 = Instant::now();
    assert!(n
        .record(NotifyKind::Bell, "a".into(), String::new(), t0)
        .is_some());
    // Same kind+pane+detail 5s later → suppressed.
    let later = t0 + Duration::from_secs(5);
    assert!(n
        .record(NotifyKind::Bell, "a".into(), String::new(), later)
        .is_none());
}

#[test]
fn same_event_surfaces_again_after_cooldown() {
    let mut n = Notifier::default();
    let t0 = Instant::now();
    assert!(n
        .record(NotifyKind::Bell, "a".into(), String::new(), t0)
        .is_some());
    let after = t0 + Duration::from_secs(11);
    assert!(n
        .record(NotifyKind::Bell, "a".into(), String::new(), after)
        .is_some());
}

#[test]
fn different_pane_is_not_throttled() {
    let mut n = Notifier::default();
    let t0 = Instant::now();
    assert!(n
        .record(NotifyKind::Bell, "a".into(), String::new(), t0)
        .is_some());
    assert!(n
        .record(NotifyKind::Bell, "b".into(), String::new(), t0)
        .is_some());
}

#[test]
fn recent_ring_is_capped() {
    let mut n = Notifier::default();
    let t0 = Instant::now();
    for i in 0..(CAP + 10) {
        // Distinct detail each time so none are throttled.
        n.record(NotifyKind::Pattern, "p".into(), i.to_string(), t0);
    }
    assert_eq!(n.len(), CAP);
}

#[test]
fn each_kind_formats_distinctly() {
    assert!(format_message(NotifyKind::AgentDone, "p", "claude").contains("finished"));
    assert!(format_message(NotifyKind::Bell, "p", "").contains("bell"));
    assert!(format_message(NotifyKind::Pattern, "p", "error").contains("error"));
    assert!(format_message(NotifyKind::Exited, "p", "").contains("exited"));
}

#[test]
fn agent_done_fires_after_the_threshold() {
    let t0 = Instant::now();
    let out = agent_done(
        Some("claude"),
        None,
        Some(t0),
        Duration::from_secs(10),
        t0 + Duration::from_secs(11),
    );
    // The command AND how long it took: a six-second build and a
    // nine-minute one are different events.
    assert_eq!(out.finished.as_deref(), Some("claude (11s)"));
    assert_eq!(out.since, None);
}

#[test]
fn agent_done_suppressed_under_the_threshold() {
    let t0 = Instant::now();
    let out = agent_done(
        Some("ls"),
        None,
        Some(t0),
        Duration::from_secs(10),
        t0 + Duration::from_secs(3),
    );
    assert_eq!(out.finished, None);
    assert_eq!(out.since, None);
}

#[test]
fn agent_done_starts_the_timer_on_launch() {
    let t0 = Instant::now();
    let out = agent_done(None, Some("claude"), None, Duration::from_secs(10), t0);
    assert_eq!(out.finished, None);
    assert_eq!(out.since, Some(t0));
}

#[test]
fn agent_done_ignores_command_to_command_changes() {
    let t0 = Instant::now();
    let out = agent_done(
        Some("cargo"),
        Some("rustc"),
        Some(t0),
        Duration::from_secs(10),
        t0 + Duration::from_secs(11),
    );
    assert_eq!(out.finished, None);
    // The original start is preserved so the whole burst is timed as one.
    assert_eq!(out.since, Some(t0));
}

#[test]
fn agent_done_without_a_recorded_start_does_not_fire() {
    let t0 = Instant::now();
    let out = agent_done(Some("claude"), None, None, Duration::from_secs(10), t0);
    assert_eq!(out.finished, None);
}
