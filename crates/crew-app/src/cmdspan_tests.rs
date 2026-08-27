use super::*;

#[test]
fn a_command_that_ran_leaves_a_span_from_its_start_to_its_end() {
    let mut s = Spans::default();
    s.started("cargo".into(), 100);
    s.close(180);
    let span = s.nth_back(0, 180).expect("a finished command has output");
    assert_eq!(
        (span.name.as_str(), span.from, span.to),
        ("cargo", 100, Some(180))
    );
    assert_eq!(Spans::range(span, 180), (100, 180));
}

/// A running command's output is worth showing before it finishes — that is
/// the case you most want `/out` for.
#[test]
fn a_running_command_reports_what_it_has_printed_so_far() {
    let mut s = Spans::default();
    s.started("cargo".into(), 10);
    assert_eq!(Spans::range(s.nth_back(0, 45).unwrap(), 45), (10, 45));
    // …but not before it has printed anything at all.
    let mut fresh = Spans::default();
    fresh.started("ls".into(), 10);
    assert!(fresh.nth_back(0, 10).is_none());
}

/// A missed end (a poll that saw two transitions at once) must not leave a
/// span that swallows everything after it.
#[test]
fn an_unclosed_span_is_closed_when_the_next_command_starts() {
    let mut s = Spans::default();
    s.started("first".into(), 10);
    s.started("second".into(), 50);
    let span = s.nth_back(0, 80).unwrap();
    assert_eq!(span.name, "second");
    assert_eq!(Spans::range(span, 80), (50, 80));
}

/// The scrollback wraps away under a span; a range past the buffer's end
/// would slice nothing at all.
#[test]
fn a_range_is_clamped_to_what_the_buffer_still_holds() {
    let mut s = Spans::default();
    s.started("cargo".into(), 100);
    s.close(500);
    let span = s.nth_back(0, 500).unwrap();
    assert_eq!(Spans::range(span, 300), (100, 300));
    assert_eq!(
        Spans::range(span, 50),
        (50, 50),
        "an empty range, not a backwards one"
    );
}

/// A close that arrives before the start (a clock or a clear moving the
/// buffer under us) must not make a backwards span.
#[test]
fn a_close_never_ends_before_its_own_start() {
    let mut s = Spans::default();
    s.started("weird".into(), 100);
    s.close(20);
    assert_eq!(
        s.nth_back(0, 100).map(|x| x.to),
        None,
        "an empty span is not output"
    );
}

#[test]
fn only_the_last_few_are_remembered() {
    let mut s = Spans::default();
    for i in 0..(CAP + 10) {
        s.started(format!("c{i}"), i * 10);
        s.close(i * 10 + 5);
    }
    assert_eq!(s.len(), CAP);
    assert_eq!(
        s.nth_back(0, usize::MAX).unwrap().name,
        format!("c{}", CAP + 9)
    );
}

#[test]
fn a_pane_that_has_run_nothing_has_nothing_to_show() {
    assert!(Spans::default().nth_back(0, 100).is_none());
}

/// The ticks are placed against the WINDOW, not the buffer: the same span is
/// a different row depending on where the pane is scrolled.
#[test]
fn a_command_start_maps_to_the_row_it_is_drawn_on() {
    let mut s = Spans::default();
    s.started("a".into(), 100);
    s.close(110);
    s.started("b".into(), 118);
    s.close(120);
    // 120 lines, a 20-row window at the bottom → showing lines 100..120.
    assert_eq!(s.start_rows(120, 20, 0), vec![0, 18]);
    // Scrolled back ten → showing 90..110: only the first start is in view.
    assert_eq!(s.start_rows(120, 20, 10), vec![10]);
    // Scrolled past both.
    assert_eq!(s.start_rows(120, 20, 60), Vec::<u16>::new());
}

/// A span older than the window shows nothing rather than pinning to row 0 —
/// a tick on the top row would claim a command started where it did not.
#[test]
fn a_start_above_the_window_is_not_pinned_to_its_top() {
    let mut s = Spans::default();
    s.started("old".into(), 5);
    s.close(6);
    assert!(s.start_rows(500, 20, 0).is_empty());
}

/// A pane keeps a few dozen commands, so `/out 3` reaches the run before the
/// three you have tried since.
#[test]
fn counting_back_walks_the_history_newest_first() {
    let mut s = Spans::default();
    for (i, name) in ["first", "second", "third"].iter().enumerate() {
        s.started((*name).to_string(), i * 10);
        s.close(i * 10 + 5);
    }
    let name = |n: usize| s.nth_back(n, 100).map(|x| x.name.clone());
    assert_eq!(name(0).as_deref(), Some("third"));
    assert_eq!(name(1).as_deref(), Some("second"));
    assert_eq!(name(2).as_deref(), Some("first"));
    assert_eq!(name(3), None, "past the history is nothing, not a wrap");
}

/// `/out` and `/out 0` must agree: a command still running with nothing
/// printed is not the latest output either way.
#[test]
fn a_silent_running_command_is_skipped_by_both_readings() {
    let mut s = Spans::default();
    s.started("old".into(), 10);
    s.close(30);
    s.started("running".into(), 30);
    assert_eq!(
        s.nth_back(0, 30).map(|x| x.name.clone()).as_deref(),
        Some("old")
    );
    assert_eq!(
        s.nth_back(0, 30).map(|x| x.name.clone()).as_deref(),
        Some("old")
    );
    assert_eq!(s.nth_back(1, 30).map(|x| x.name.clone()).as_deref(), None);
    // …and once it prints, both see it.
    assert_eq!(
        s.nth_back(0, 45).map(|x| x.name.clone()).as_deref(),
        Some("running")
    );
    assert_eq!(
        s.nth_back(0, 45).map(|x| x.name.clone()).as_deref(),
        Some("running")
    );
    assert_eq!(
        s.nth_back(1, 45).map(|x| x.name.clone()).as_deref(),
        Some("old")
    );
}

/// The summary is what `/out` says when asked for something that is not
/// there: newest first, numbered the way the argument is.
#[test]
fn the_summary_numbers_what_the_argument_would_reach() {
    let mut s = Spans::default();
    for name in ["a", "b", "c"] {
        s.started(name.to_string(), 0);
        s.close(1);
    }
    assert_eq!(s.summary(4), vec!["0:c", "1:b", "2:a"]);
    assert_eq!(s.summary(2), vec!["0:c", "1:b"]);
    assert!(Spans::default().summary(4).is_empty());
}
