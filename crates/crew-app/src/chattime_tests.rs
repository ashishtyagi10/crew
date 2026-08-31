use super::*;

#[test]
fn rel_time_buckets() {
    let now = 1_000_000_000;
    assert_eq!(rel_time("999995000", now).unwrap(), "now");
    assert_eq!(rel_time("999958000", now).unwrap(), "42s ago");
    assert_eq!(rel_time("999700000", now).unwrap(), "5m ago");
    assert_eq!(rel_time("989200000", now).unwrap(), "3h ago");
    assert_eq!(rel_time("827200000", now).unwrap(), "2d ago");
}

#[test]
fn rel_time_rejects_non_numeric_ts() {
    assert_eq!(rel_time("", 1000), None);
    assert_eq!(rel_time("t", 1000), None);
}

#[test]
fn task_tag_parses_the_leading_id() {
    assert_eq!(task_tag("task:3"), Some(3));
    assert_eq!(task_tag("task:3 \u{00b7} 0.0s"), Some(3));
    assert_eq!(task_tag(""), None);
    assert_eq!(task_tag("4.2s"), None);
    assert_eq!(task_tag("task:"), None);
    assert_eq!(task_tag("task:abc"), None);
}

#[test]
fn strip_task_tag_keeps_only_the_latency() {
    assert_eq!(strip_task_tag("task:3 \u{00b7} 0.0s"), "0.0s");
    assert_eq!(strip_task_tag("task:3"), "");
    assert_eq!(strip_task_tag("4.2s"), "4.2s"); // untagged unchanged
    assert_eq!(strip_task_tag(""), "");
}
