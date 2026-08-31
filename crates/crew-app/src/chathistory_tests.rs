use super::*;

fn hist(lines: &[&str]) -> History {
    let mut h = History::default();
    for l in lines {
        h.record(l);
    }
    h
}

#[test]
fn up_walks_back_and_stops_at_the_oldest() {
    let mut h = hist(&["one", "two"]);
    let mut s = String::new();
    assert!(h.prev(&mut s));
    assert_eq!(s, "two");
    assert!(h.prev(&mut s));
    assert_eq!(s, "one");
    // Oldest: held, not wrapped round to the newest.
    assert!(!h.prev(&mut s));
    assert_eq!(s, "one");
}

#[test]
fn down_walks_forward_and_restores_what_was_typed() {
    let mut h = hist(&["one", "two"]);
    let mut s = String::new();
    h.prev(&mut s);
    h.prev(&mut s);
    assert_eq!(s, "one");
    assert!(h.next(&mut s));
    assert_eq!(s, "two");
    assert!(h.next(&mut s));
    assert_eq!(s, "", "past the newest is the empty composer again");
    assert!(!h.next(&mut s), "already live");
}

/// The input bar's rule, in the composer: typed text filters the walk
/// instead of being thrown away by it.
#[test]
fn typed_text_filters_the_recall_and_comes_back() {
    let mut h = hist(&["/model list", "run the tests", "/model all qwen-max"]);
    let mut s = "/model".to_string();
    assert!(h.prev(&mut s));
    assert_eq!(s, "/model all qwen-max");
    assert!(h.prev(&mut s), "skips the non-matching entry between them");
    assert_eq!(s, "/model list");
    assert!(!h.prev(&mut s), "no older /model");
    h.next(&mut s);
    h.next(&mut s);
    assert_eq!(s, "/model", "the typed prefix is restored, not lost");
}

#[test]
fn a_prefix_matching_nothing_leaves_the_composer_alone() {
    let mut h = hist(&["one", "two"]);
    let mut s = "zzz".to_string();
    assert!(!h.prev(&mut s));
    assert_eq!(s, "zzz");
}

#[test]
fn down_from_live_input_does_nothing() {
    let mut h = hist(&["one"]);
    let mut s = "typing".to_string();
    assert!(!h.next(&mut s));
    assert_eq!(s, "typing");
}

#[test]
fn empty_history_recalls_nothing() {
    let mut h = History::default();
    let mut s = "typing".to_string();
    assert!(!h.prev(&mut s));
    assert_eq!(s, "typing");
}

#[test]
fn editing_a_recalled_line_makes_it_the_draft() {
    let mut h = hist(&["one", "two"]);
    let mut s = String::new();
    h.prev(&mut s);
    s.push('!'); // the caller's edit
    h.edited();
    // Down must not resurrect the old prefix over what was just typed.
    assert!(!h.next(&mut s));
    assert_eq!(s, "two!");
}

#[test]
fn blanks_and_immediate_repeats_are_not_recorded() {
    let mut h = hist(&["one", "one", "  ", "", "two"]);
    let mut s = String::new();
    h.prev(&mut s);
    assert_eq!(s, "two");
    h.prev(&mut s);
    assert_eq!(s, "one");
    assert!(!h.prev(&mut s), "only two entries should exist");
}

#[test]
fn a_repeat_that_is_not_immediate_is_kept() {
    let mut h = hist(&["one", "two", "one"]);
    let mut s = String::new();
    h.prev(&mut s);
    assert_eq!(s, "one");
    h.prev(&mut s);
    assert_eq!(s, "two");
}

#[test]
fn recording_returns_the_arrows_to_the_newest() {
    let mut h = hist(&["one", "two"]);
    let mut s = String::new();
    h.prev(&mut s);
    h.prev(&mut s);
    assert_eq!(s, "one");
    h.record("three"); // sending from mid-history
    let mut s = String::new();
    assert!(h.prev(&mut s));
    assert_eq!(s, "three", "Up starts from the newest again");
}

#[test]
fn the_oldest_entries_fall_off_at_the_cap() {
    let mut h = History::default();
    for i in 0..CAP + 10 {
        h.record(&format!("p{i}"));
    }
    assert_eq!(h.lines.len(), CAP);
    let mut s = String::new();
    h.prev(&mut s);
    assert_eq!(s, format!("p{}", CAP + 9), "newest survives");
    // Walking to the end reaches the oldest entry still held, not p0.
    while h.prev(&mut s) {}
    assert_eq!(s, "p10");
}
