//! What counts as a check run.
use super::*;

#[test]
fn a_runner_named_for_a_check_alone_or_after_and_is_a_check() {
    assert!(is_check("cargo test -p crew-hive"));
    assert!(is_check("cd crates/x && cargo build"));
    assert!(is_check("RUST_LOG=1 npm run lint"));
    assert!(is_check("pytest -q"));
}

#[test]
fn a_command_with_no_runner_or_no_reason_is_not() {
    assert!(!is_check("ls src"));
    assert!(!is_check("cat Cargo.toml && grep cargo README.md"));
    assert!(!is_check("cargo add serde"));
    assert!(!is_check("npm install"));
    assert!(!is_check(""));
}

#[test]
fn a_check_whose_exit_a_later_command_owns_is_not() {
    assert!(!is_check("cargo test 2>&1 | tail -20"));
    assert!(!is_check("cargo test; echo ok"));
    assert!(!is_check("cargo test || true"));
    assert!(!is_check("cargo test\necho ok"));
}
