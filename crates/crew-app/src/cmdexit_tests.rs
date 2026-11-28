use super::*;

/// A command the poll saw: opened, closed, with the shell's `C` before the
/// poll noticed it (C at the start, the span a poll later).
fn seen(s: &mut Spans, name: &str, (c, open, close): (u64, u64, u64), at: usize) {
    s.output_started(c);
    s.started(name.into(), at, open);
    s.close_at(at + 5, close);
}

#[test]
fn a_normal_command_is_stamped() {
    let mut s = Spans::default();
    seen(&mut s, "cargo", (1_000, 2_000, 9_000), 10);
    s.finished(Some(101), 16, 9_001);
    assert_eq!(s.recent().next().unwrap().exit, Some(101));
}

#[test]
fn a_fast_success_after_a_failure_leaves_the_failure_alone() {
    let mut s = Spans::default();
    seen(&mut s, "cargo", (1_000, 2_000, 9_000), 10);
    s.finished(Some(101), 16, 9_001);
    // `ls`: C and D, no poll between, so no span.
    s.output_started(9_500);
    s.finished(Some(0), 20, 9_501);
    assert_eq!(s.recent().next().unwrap().exit, Some(101));
    assert_eq!(s.last_exit(), Some(101));
}

#[test]
fn a_fast_failure_after_a_success_is_blamed_on_nobody() {
    let mut s = Spans::default();
    seen(&mut s, "make", (1_000, 2_000, 9_000), 10);
    s.close_at(15, 9_000);
    // The `D` for `make` is late: after the poll, but its `C` was before.
    s.finished(Some(0), 16, 9_001);
    s.output_started(9_500);
    s.finished(Some(1), 20, 9_501);
    let span = s.recent().next().unwrap();
    assert_eq!(span.exit, Some(0), "the success is not rewritten");
    assert!(s.recent().all(|s| s.exit != Some(1)), "no one is blamed");
}

/// A shell that never sends `C` has only the once-only rule to lean on.
#[test]
fn without_any_c_a_second_d_is_still_ignored() {
    let mut s = Spans::default();
    s.started("cargo".into(), 0, 0);
    s.finished(Some(101), 5, 1);
    s.finished(Some(0), 9, 2);
    assert_eq!(s.last_exit(), Some(101));
}

/// A span that never got its own `D` (closed, exit unknown) must not take a
/// later span-less command's: that command's `C` came after the span closed.
#[test]
fn a_d_from_a_command_that_started_after_the_span_closed_is_dropped() {
    let mut s = Spans::default();
    seen(&mut s, "make", (1_000, 2_000, 9_000), 10);
    s.output_started(9_500);
    s.finished(Some(1), 20, 9_501);
    assert_eq!(s.last_exit(), None, "nobody is blamed");
    // The same span's own `D`, whose `C` was before it closed, still lands.
    let mut t = Spans::default();
    seen(&mut t, "make", (1_000, 2_000, 9_000), 10);
    t.finished(Some(1), 20, 9_001);
    assert_eq!(t.last_exit(), Some(1));
}
