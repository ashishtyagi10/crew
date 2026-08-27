use super::*;

#[test]
fn a_command_that_ran_leaves_a_span_from_its_start_to_its_end() {
    let mut s = Spans::default();
    s.started("cargo".into(), 100);
    s.close(180);
    let span = s.latest(180).expect("a finished command has output");
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
    assert_eq!(Spans::range(s.latest(45).unwrap(), 45), (10, 45));
    // …but not before it has printed anything at all.
    let mut fresh = Spans::default();
    fresh.started("ls".into(), 10);
    assert!(fresh.latest(10).is_none());
}

/// A missed end (a poll that saw two transitions at once) must not leave a
/// span that swallows everything after it.
#[test]
fn an_unclosed_span_is_closed_when_the_next_command_starts() {
    let mut s = Spans::default();
    s.started("first".into(), 10);
    s.started("second".into(), 50);
    let span = s.latest(80).unwrap();
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
    let span = s.latest(500).unwrap();
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
        s.latest(100).map(|x| x.to),
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
    assert_eq!(s.latest(usize::MAX).unwrap().name, format!("c{}", CAP + 9));
}

#[test]
fn a_pane_that_has_run_nothing_has_nothing_to_show() {
    assert!(Spans::default().latest(100).is_none());
}
