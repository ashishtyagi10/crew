//! Exit codes are the daemon CLI's whole interface for scripts (`crew daemon status || …`), and
//! the argument parsing decides which session a `close` lands on, so both are pinned here.
use super::{flag, positional, run_sub};

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_missing_or_unknown_subcommand_prints_usage_and_fails() {
    assert_eq!(run_sub(&args(&[])), 2);
    assert_eq!(run_sub(&args(&["strt"])), 2);
    assert_eq!(run_sub(&args(&["--help"])), 2);
}

/// `close` with no id must not fall through to closing something arbitrary.
#[test]
fn close_without_an_id_is_a_usage_error() {
    assert_eq!(run_sub(&args(&["close"])), 2);
}

#[test]
fn a_flag_reads_the_value_that_follows_it() {
    assert_eq!(
        flag(&args(&["open", "--cwd", "/tmp"]), "--cwd"),
        Some("/tmp")
    );
    assert_eq!(flag(&args(&["open"]), "--cwd"), None);
    // A flag with nothing after it has no value, rather than swallowing the next subcommand.
    assert_eq!(flag(&args(&["open", "--cwd"]), "--cwd"), None);
}

/// The label is positional, so it must not pick up a flag or a flag's value — `crew daemon open
/// --cwd /tmp` labels the session "crew", not "/tmp".
#[test]
fn the_positional_label_skips_flags_and_their_values() {
    assert_eq!(positional(&args(&["open", "--cwd", "/tmp"]), 1), None);
    assert_eq!(positional(&args(&["open", "smith"]), 1), Some("smith"));
    assert_eq!(
        positional(&args(&["open", "--cwd", "/tmp", "smith"]), 1),
        Some("smith")
    );
}
