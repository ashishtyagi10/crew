use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A source that says `said` on its first call and nothing after, counting
/// how often it was asked.
fn once(said: &[&str]) -> (Steers, Arc<AtomicUsize>) {
    let asked = Arc::new(AtomicUsize::new(0));
    let (n, said): (_, Vec<String>) = (
        Arc::clone(&asked),
        said.iter().map(|s| s.to_string()).collect(),
    );
    let steers = Steers::new(move || match n.fetch_add(1, Ordering::SeqCst) {
        0 => said.clone(),
        _ => Vec::new(),
    });
    (steers, asked)
}

#[test]
fn what_one_worker_takes_every_worker_reads() {
    let (a, _) = once(&["also check the tests"]);
    let b = a.clone();
    assert_eq!(a.all(), ["also check the tests"]);
    // The source is empty now; the other worker still reads it from the log.
    assert_eq!(b.all(), ["also check the tests"]);
    assert_eq!(a.all(), ["also check the tests"], "and reads it again");
}

#[test]
fn taken_reads_the_log_without_asking_the_source() {
    let (s, asked) = once(&["one"]);
    assert!(s.taken().is_empty());
    assert_eq!(
        asked.load(Ordering::SeqCst),
        0,
        "nothing taken for the answer"
    );
    s.all();
    assert_eq!(s.taken(), ["one"]);
    assert_eq!(asked.load(Ordering::SeqCst), 1);
}

#[test]
fn a_blank_steer_is_not_logged() {
    let (s, _) = once(&["  ", "real"]);
    assert_eq!(s.all(), ["real"]);
}

#[test]
fn the_section_heads_the_items_and_ends_on_its_tail() {
    assert_eq!(section(&[], WORKER_TAIL), "");
    let s = section(&["a".into(), "b\nmore".into()], "TAIL");
    assert_eq!(s, format!("{HEAD}\n- a\n- b\n  more\nTAIL\n\n"));
}
