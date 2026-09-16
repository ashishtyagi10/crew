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

#[test]
fn backspace_on_an_empty_composer_takes_the_last_queued_message_back() {
    let mut p = pane();
    p.queued.push_back("first".into());
    p.queued.push_back("second".into());
    assert!(take_back(&mut p));
    // Back where it was typed, so it can be edited and sent again — nothing
    // is destroyed by a keystroke.
    assert_eq!(p.input, "second");
    assert_eq!(p.queued.len(), 1, "it took only one");
    assert_eq!(p.queued.front().map(String::as_str), Some("first"));
}

#[test]
fn it_never_eats_something_half_typed() {
    // The guard the whole gesture rests on: with text in the composer,
    // backspace is deleting that.
    let mut p = pane();
    p.queued.push_back("queued".into());
    p.input.push_str("half a thou");
    assert!(!take_back(&mut p));
    assert_eq!(p.input, "half a thou");
    assert_eq!(p.queued.len(), 1);
}

#[test]
fn with_nothing_queued_it_does_nothing_and_backspace_stays_backspace() {
    let mut p = pane();
    assert!(!take_back(&mut p));
    assert_eq!(p.input, "");
}

#[test]
fn taking_them_all_back_empties_the_queue_newest_first() {
    let mut p = pane();
    for t in ["a", "b", "c"] {
        p.queued.push_back(t.into());
    }
    let mut got = Vec::new();
    while take_back(&mut p) {
        got.push(std::mem::take(&mut p.input));
    }
    assert_eq!(
        got,
        ["c", "b", "a"],
        "backspace undoes the last thing first"
    );
    assert!(p.queued.is_empty());
    assert_eq!(queued_rows(&p), 0, "and the indicator goes with it");
}

#[test]
fn the_backspace_key_itself_reaches_the_queue() {
    // The guard lives in `chattype`'s match; without this test the whole
    // gesture could be dead while every unit below it passed.
    use crate::chatkeys::ChatInput;
    let cwd = std::path::PathBuf::from(".");
    let mut p = pane();
    p.queued.push_back("take me back".into());
    assert!(p.on_input(ChatInput::Backspace, &cwd).is_none());
    assert_eq!(p.input, "take me back");
    assert!(p.queued.is_empty());
    // A second one has nothing to take, and deletes from the composer as
    // backspace always has.
    assert!(p.on_input(ChatInput::Backspace, &cwd).is_none());
    assert_eq!(p.input, "take me bac");
}
