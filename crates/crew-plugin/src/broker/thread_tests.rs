use super::*;

#[path = "thread_arms_tests.rs"]
mod arms;
#[path = "thread_swarm_tests.rs"]
mod swarm;

const HEAD: &str = "Earlier in this conversation:";

fn filled(n: usize) -> Thread {
    let mut t = Thread::default();
    for i in 0..n {
        t.record(&format!("ask {i}"), &format!("answer {i}"));
    }
    t
}

fn asked(t: &Thread) -> Vec<String> {
    t.turns().map(|x| x.asked.clone()).collect()
}

#[test]
fn the_thread_keeps_the_last_six_turns_and_drops_the_oldest() {
    let t = filled(8);
    assert_eq!(t.len(), TURNS_MAX);
    assert_eq!(asked(&t).first().map(String::as_str), Some("ask 2"));
    assert_eq!(asked(&t).last().map(String::as_str), Some("ask 7"));
}

#[test]
fn a_thread_over_the_byte_cap_drops_the_oldest_first_and_keeps_the_newest() {
    let mut t = Thread::default();
    let long = "x".repeat(BYTES_MAX / 2 - 100); // three of these bust the cap
    t.record("first", &long);
    t.record("second", &long);
    assert_eq!(t.len(), 2);
    t.record("third", &long);
    assert_eq!(asked(&t), vec!["second", "third"]);
}

#[test]
fn one_runaway_answer_is_clipped_at_record_time_with_a_visible_marker() {
    let mut t = Thread::default();
    t.record("q", &"y".repeat(ANSWER_CAP + 500));
    let kept = &t.turns().next().unwrap().answered;
    assert!(kept.ends_with("\u{2026} [clipped 500 chars]"), "{kept}");
    assert_eq!(
        kept.chars().count(),
        ANSWER_CAP + "… [clipped 500 chars]".chars().count()
    );
}

#[test]
fn an_empty_side_is_not_a_turn() {
    let mut t = Thread::default();
    t.record("q", "   ");
    t.record("", "a");
    assert_eq!(t.len(), 0);
}

#[test]
fn an_empty_thread_has_no_context_and_leaves_the_task_byte_identical() {
    assert_eq!(Thread::default().context(CONTEXT_CAP), None);
    let shared: SharedThread = Default::default();
    assert_eq!(
        with_context(&shared, "plan the release"),
        "plan the release"
    );
}

#[test]
fn the_context_carries_asked_and_answered_newest_last_before_the_task() {
    let shared: SharedThread = Default::default();
    lock(&shared).record("list the crates", "crew-app, crew-hive, crew-plugin");
    lock(&shared).record("which is largest?", "crew-app");
    let framed = with_context(&shared, "and the smallest?");
    let block = lock(&shared).context(CONTEXT_CAP).unwrap();
    assert!(block.starts_with(HEAD), "{block}");
    let first = block.find("you asked: list the crates").unwrap();
    let second = block.find("you asked: which is largest?").unwrap();
    assert!(first < second, "newest last: {block}");
    assert!(block.contains("crew answered: crew-app, crew-hive, crew-plugin"));
    assert!(framed.ends_with("\n\nNow:\nand the smallest?"), "{framed}");
    assert!(framed.starts_with(&block), "{framed}");
}

#[test]
fn a_context_over_budget_is_clipped_with_a_marker_and_never_exceeds_the_budget() {
    let mut t = Thread::default();
    t.record("q", &"z".repeat(3000));
    let block = t.context(CONTEXT_CAP).unwrap();
    assert!(block.chars().count() <= CONTEXT_CAP, "{}", block.len());
    assert!(block.contains("\u{2026} [clipped "), "{block}");
    let t = filled(6);
    let block = t.context(200).unwrap();
    assert!(block.chars().count() <= 200, "{}", block.len());
}

#[test]
fn turns_that_cannot_fit_are_counted_not_silently_dropped() {
    let mut t = Thread::default();
    for i in 0..4 {
        t.record(&format!("question number {i}"), &"w".repeat(600));
    }
    let block = t.context(400).unwrap();
    assert!(block.contains("earlier turn(s) omitted]"), "{block}");
    assert!(
        block.contains("question number 3"),
        "the newest survives: {block}"
    );
}

#[test]
fn the_recent_line_is_the_last_request_on_one_line_and_clipped() {
    let mut t = Thread::default();
    t.record("first", "a");
    t.record(&format!("now\nthe {}", "same ".repeat(60)), "b");
    let r = t.recent().unwrap();
    assert!(r.starts_with("now the same"), "{r}");
    assert!(!r.contains('\n') && r.ends_with('\u{2026}'), "{r}");
    assert!(r.chars().count() <= RECENT_CAP + 1, "{}", r.len());
    assert_eq!(Thread::default().recent(), None);
}

#[test]
fn stop_clears_every_turn() {
    let mut t = filled(3);
    t.clear();
    assert_eq!(t.len(), 0);
    assert_eq!(t.context(CONTEXT_CAP), None);
}

#[test]
fn a_fans_replies_combine_one_plain_several_named_none_not_at_all() {
    assert_eq!(combined(vec![]), None);
    assert_eq!(
        combined(vec![("a".into(), "hi".into())]).as_deref(),
        Some("hi")
    );
    let two = combined(vec![("a".into(), "hi".into()), ("b".into(), "yo".into())]);
    assert_eq!(two.as_deref(), Some("a: hi\n\nb: yo"));
}

#[test]
fn the_doctor_line_reads_zero_then_one_turn() {
    assert_eq!(doctor_line(0), ('\u{2013}', "0 turns remembered".into()));
    assert_eq!(doctor_line(1), ('\u{2713}', "1 turn remembered".into()));
}
