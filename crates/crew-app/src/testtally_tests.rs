use super::{tally, Tally};

const CARGO: &str = include_str!("../../crew-hive/src/testdata/check_cargo_test.txt");

#[test]
fn cargo_counts_and_names_what_failed() {
    let t = tally(CARGO).unwrap();
    assert_eq!((t.passed, t.failed), (39, 1));
    assert_eq!(t.names, vec!["broker::y".to_string()]);
}

/// A workspace run prints a `test result:` per binary; the run is their sum.
#[test]
fn cargo_sums_every_test_binary() {
    let out = "running 3 tests\ntest result: ok. 3 passed; 0 failed; 0 ignored\n\n\
               running 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored\n\n\
               running 5 tests\ntest result: FAILED. 4 passed; 1 failed; 2 ignored\n";
    let t = tally(out).unwrap();
    assert_eq!((t.passed, t.failed, t.ran()), (7, 1, 8));
}

/// The case the exit code hides: a filter that matched nothing exits 0.
#[test]
fn a_cargo_filter_that_matched_nothing_ran_zero() {
    let out = "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; \
               0 measured; 42 filtered out; finished in 0.00s\n";
    assert_eq!(tally(out), Some(Tally::default()));
}

#[test]
fn pytest_jest_and_vitest_summaries() {
    let py = "tests/test_x.py ..F.\n==== 1 failed, 3 passed in 0.12s ====\n";
    assert_eq!(tally(py).map(|t| (t.passed, t.failed)), Some((3, 1)));
    let none = "===== no tests ran in 0.01s =====";
    assert_eq!(tally(none).map(|t| t.ran()), Some(0));
    let jest =
        "Test Suites: 1 failed, 4 passed, 5 total\nTests:       2 failed, 40 passed, 42 total\n";
    assert_eq!(tally(jest).map(|t| (t.passed, t.failed)), Some((40, 2)));
    let vitest = " Test Files  1 passed (1)\n      Tests  12 passed (12)\n";
    assert_eq!(tally(vitest).map(|t| (t.passed, t.failed)), Some((12, 0)));
}

#[test]
fn output_with_no_summary_is_none() {
    assert_eq!(tally("Compiling crew v0.1.0\nFinished dev profile"), None);
    assert_eq!(tally(""), None);
}
