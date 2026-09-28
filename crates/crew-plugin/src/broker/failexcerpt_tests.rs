use super::*;

/// `cargo build` as `sys:run` returns it: sixty crates compiling on stderr
/// (`thiserror` and `quick-error` among them), then one type error.
const BUILD: &str = include_str!("testdata/check_cargo_build.txt");
/// `cargo test` as `sys:run` returns it: forty tests on stdout, a third of
/// them named with `error`, `failed`, `expected` or `assert`, one of them
/// failing, and cargo's own verdict on stderr after the summary.
const TEST: &str = include_str!("testdata/check_cargo_test.txt");

fn count(s: &str, what: &str) -> usize {
    s.lines().filter(|l| l.contains(what)).count()
}

#[test]
fn a_build_failure_is_the_error_and_where_it_is_not_the_compiling() {
    let e = excerpt(BUILD);
    assert!(e.contains("error[E0308]: mismatched types"), "{e}");
    assert!(e.contains(" --> src/lib.rs:12:5"), "{e}");
    assert!(e.contains("error: could not compile `app`"), "{e}");
    assert!(count(&e, "Compiling") <= 3, "the progress came back: {e}");
    assert!(e.starts_with("exit 101\n"), "the exit code went: {e}");
}

#[test]
fn a_test_failure_is_the_panic_its_values_and_the_summary() {
    let e = excerpt(TEST);
    assert!(
        e.contains("thread 'broker::y' panicked at src/a.rs:9:5:"),
        "{e}"
    );
    assert!(e.contains("assertion `left == right` failed"), "{e}");
    assert!(e.contains("  left: 1") && e.contains(" right: 2"), "{e}");
    assert!(
        e.contains("test result: FAILED. 39 passed; 1 failed"),
        "{e}"
    );
    assert!(e.contains("test broker::y ... FAILED"), "{e}");
}

#[test]
fn a_passing_test_is_not_a_failure_whatever_it_is_called() {
    // Twelve of the thirty-nine passing tests are named with a failure word.
    // Picked, each would bring two more passing lines with it.
    let e = excerpt(TEST);
    let passing = count(&e, "... ok");
    assert!(passing <= 5, "{passing} passing tests were picked: {e}");
    assert!(!e.contains("t00_an_error_line_is_kept"), "{e}");
}

#[test]
fn skipped_lines_are_marked_where_they_were() {
    let e = excerpt(BUILD);
    let lines: Vec<&str> = e.lines().collect();
    // The gap sits exactly where the sixty `Compiling` lines were: after
    // the stderr marker, before the error.
    assert_eq!(lines.get(1), Some(&"--- stderr ---"), "{e}");
    assert_eq!(lines.get(2), Some(&GAP), "{e}");
    assert_eq!(lines.get(3), Some(&"error[E0308]: mismatched types"), "{e}");
    assert!(
        lines.windows(2).all(|w| !(w[0] == GAP && w[1] == GAP)),
        "two gap marks in a row: {e}"
    );
}

#[test]
fn nothing_that_names_a_failure_is_answered_with_the_end() {
    let steps: String = (0..100).map(|i| format!("step {i} done\n")).collect();
    let e = excerpt(&format!("exit 1\n{steps}"));
    assert!(e.contains("step 99 done"), "{e}");
    assert!(e.contains("step 60 done"), "{e}");
    assert!(!e.contains("step 59 done"), "more than the last 40: {e}");
    assert!(!e.contains("step 0 done"), "the head came back: {e}");
    assert!(e.starts_with("exit 1\n\u{2026}\n"), "{e}");
}

#[test]
fn warnings_are_the_failure_only_when_nothing_else_is() {
    let only = "exit 1\nstep one\nwarning: unused variable: `x`\nstep two\n";
    assert!(excerpt(only).contains("warning: unused variable"));
    assert_eq!(headline(only, 6), vec!["warning: unused variable: `x`"]);
    // A warning that mentions an error is still a warning.
    let both = "exit 1\nwarning: unused import: `std::error::Error`\nerror: boom\n";
    assert_eq!(headline(both, 6), vec!["error: boom"]);
}

#[test]
fn the_verdict_names_the_failing_test_and_the_summary_in_six_lines() {
    let v = headline(TEST, 6);
    assert!(v.len() <= 6, "{v:?}");
    assert!(v.contains(&"test broker::y ... FAILED"), "{v:?}");
    assert_eq!(
        v.last().copied(),
        Some("test result: FAILED. 39 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s"),
        "the summary is not last: {v:?}"
    );
}

#[test]
fn the_verdict_on_a_build_is_the_error_its_place_and_the_last_word() {
    let v = headline(BUILD, 6);
    assert_eq!(v.len(), 6, "{v:?}");
    assert_eq!(v[0], "error[E0308]: mismatched types");
    assert_eq!(v[1], " --> src/lib.rs:12:5");
    assert_eq!(
        v[5],
        "error: could not compile `app` (lib) due to 1 previous error"
    );
    assert!(v.iter().all(|l| !l.contains("Compiling")), "{v:?}");
}

#[test]
fn a_failure_word_inside_a_name_is_not_one() {
    assert!(failure("FAIL src/a.test.js"));
    assert!(failure("--- FAIL: TestParse (0.00s)"));
    assert!(!failure("running test_fail_path"));
    assert!(progress("   Compiling thiserror v1.0.69"));
    assert!(!failure("tests/test_a.py::test_error_path PASSED [ 10%]"));
    assert!(failure("tests/test_a.py::test_error_path FAILED [ 10%]"));
}
