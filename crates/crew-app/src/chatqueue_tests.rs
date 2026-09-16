use super::*;
use crate::chat::ChatPane;
use crew_plugin::Plugin;

fn pane() -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

#[test]
fn stop_bypasses_the_queue_regardless_of_spacing() {
    assert!(is_stop("/stop"));
    assert!(is_stop("  /stop  "));
    assert!(is_stop("/stop #2"));
    assert!(!is_stop("/stopwatch"));
    assert!(!is_stop("hello /stop"));
}

#[test]
fn queued_rows_is_the_summary_plus_one_per_message_waiting() {
    let mut p = pane();
    assert_eq!(queued_rows(&p), 0);
    p.queued.push_back("hi".into());
    assert_eq!(queued_rows(&p), 2, "the summary and the one message");
    p.queued.push_back("there".into());
    assert_eq!(queued_rows(&p), 3);
    // Past SHOWN the rest become one line, so the queue can never push the
    // conversation off the top however deep it gets.
    for i in 0..20 {
        p.queued.push_back(format!("m{i}"));
    }
    assert_eq!(queued_rows(&p), 1 + SHOWN as u16 + 1);
}

#[test]
fn a_pasted_block_is_listed_as_one_line() {
    // It is one thing waiting, and it claims one row like everything else.
    let mut p = pane();
    p.queued
        .push_back("first line\nsecond line\n\nfourth".into());
    assert_eq!(queued_rows(&p), 2);
    assert_eq!(listed(&p), vec!["1. first line second line fourth"]);
}

#[test]
fn past_shown_the_rest_of_the_queue_becomes_one_count() {
    let mut p = pane();
    for i in 1..=7 {
        p.queued.push_back(format!("task {i}"));
    }
    let rows = listed(&p);
    assert_eq!(rows.len(), SHOWN + 1);
    assert_eq!(rows[0], "1. task 1");
    assert_eq!(rows[SHOWN], "\u{2026} +4 more");
}
