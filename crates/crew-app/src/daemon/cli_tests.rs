//! Exit codes are the daemon CLI's whole interface for scripts (`crew daemon status || …`),
//! so they are pinned here.
use super::run_sub;

#[test]
fn a_missing_or_unknown_subcommand_prints_usage_and_fails() {
    assert_eq!(run_sub(None), 2);
    assert_eq!(run_sub(Some("strt")), 2);
    assert_eq!(run_sub(Some("--help")), 2);
}
