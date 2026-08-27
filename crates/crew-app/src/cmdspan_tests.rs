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

/// Which command owns a given buffer line — the lookup behind the name the
/// card's top border carries while a pane is scrolled back.
#[test]
fn a_line_is_answered_by_the_command_that_printed_it() {
    let mut s = Spans::default();
    s.started("cargo build".into(), 10);
    s.close(40);
    s.started("cargo test".into(), 40);
    s.close(90);
    let name = |line| s.at_line(line, 100).map(|x| x.name.clone());
    assert_eq!(name(10).as_deref(), Some("cargo build"), "its first line");
    assert_eq!(name(39).as_deref(), Some("cargo build"), "its last line");
    assert_eq!(
        name(40).as_deref(),
        Some("cargo test"),
        "the next one's first"
    );
    assert_eq!(name(89).as_deref(), Some("cargo test"));
    // Before the first command crew saw, and after the last one ended:
    // there is no honest answer, so there is no answer.
    assert_eq!(name(0), None, "before anything crew watched");
    assert_eq!(name(90), None, "at the live prompt");
}

/// The poll's one-second granularity can close a span a line or two into the
/// next command's output. Where two spans overlap, the LATER one wins: the
/// top of the window is inside the thing that printed most recently.
#[test]
fn overlapping_spans_answer_with_the_later_command() {
    let mut s = Spans::default();
    s.0.push(crate::cmdspan::Span {
        name: "first".into(),
        from: 0,
        to: Some(50),
        exit: None,
    });
    s.0.push(crate::cmdspan::Span {
        name: "second".into(),
        from: 40,
        to: Some(80),
        exit: None,
    });
    assert_eq!(
        s.at_line(45, 100).map(|x| x.name.clone()).as_deref(),
        Some("second")
    );
    assert_eq!(
        s.at_line(20, 100).map(|x| x.name.clone()).as_deref(),
        Some("first")
    );
}

/// A still-running command owns everything down to the live bottom, and the
/// range is clamped to the buffer that is actually there — the scrollback
/// wraps away under us.
#[test]
fn a_running_command_owns_the_lines_it_is_still_printing() {
    let mut s = Spans::default();
    s.started("cargo build".into(), 10);
    assert_eq!(
        s.at_line(30, 40).map(|x| x.name.clone()).as_deref(),
        Some("cargo build")
    );
    assert_eq!(s.at_line(40, 40), None, "past the end of the buffer");
}

/// OSC 133 is the one thing crew cannot derive: a process it never saw start
/// tells it nothing about how the command ENDED.
#[test]
fn a_shell_that_reports_an_exit_status_marks_that_block() {
    let mut s = Spans::default();
    s.started("cargo build".into(), 10);
    s.finished(Some(1), 40);
    let span = s.nth_back(0, 60).expect("the block");
    assert_eq!(span.exit, Some(1));
    assert_eq!(span.to, Some(40), "the shell's boundary closed it");
    assert_eq!(s.failed_rows(60, 60, 0), vec![10]);
}

/// A success is not a failure, and neither is a shell that said nothing —
/// which is not the same as "it succeeded", and is why nothing is drawn.
#[test]
fn only_a_reported_failure_is_marked() {
    let mut ok = Spans::default();
    ok.started("ls".into(), 0);
    ok.finished(Some(0), 4);
    assert!(ok.failed_rows(10, 10, 0).is_empty());

    let mut silent = Spans::default();
    silent.started("ls".into(), 0);
    silent.close(4);
    assert!(silent.failed_rows(10, 10, 0).is_empty());
    assert_eq!(silent.nth_back(0, 10).unwrap().exit, None);
}

/// A `D` arriving just after the foreground-process watch already closed the
/// span is the SAME command — dropping it would throw away the one fact
/// polling cannot supply.
#[test]
fn a_status_arriving_after_the_poll_closed_the_span_still_lands() {
    let mut s = Spans::default();
    s.started("cargo test".into(), 0);
    s.close(20); // the poll noticed first
    s.finished(Some(101), 21);
    let span = s.nth_back(0, 30).unwrap();
    assert_eq!(span.exit, Some(101));
    assert_eq!(span.to, Some(20), "the earlier boundary is kept");
}

#[test]
fn a_status_with_no_span_to_attach_it_to_is_dropped() {
    let mut s = Spans::default();
    s.finished(Some(1), 5);
    assert_eq!(s.len(), 0);
}

/// Failed rows use the same window arithmetic the start ticks do, so the two
/// ladders can never disagree about where a block begins.
#[test]
fn a_failure_tick_lands_on_the_same_row_its_start_tick_would() {
    let mut s = Spans::default();
    s.started("boom".into(), 100);
    s.finished(Some(2), 120);
    for scroll in 0..40 {
        assert_eq!(
            s.failed_rows(200, 50, scroll),
            s.start_rows(200, 50, scroll),
            "at scroll {scroll}"
        );
    }
}
