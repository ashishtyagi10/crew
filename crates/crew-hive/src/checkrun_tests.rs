//! What counts as a check run.
use super::*;

#[test]
fn a_runner_anywhere_in_the_chain_is_a_check() {
    assert!(is_check("cargo test -p crew-hive"));
    assert!(is_check("cd crates/x && cargo build"));
    assert!(is_check("RUST_LOG=1 cargo clippy --all-targets"));
    assert!(is_check("pytest -q | tail -5"));
}

#[test]
fn a_command_with_no_runner_is_not() {
    assert!(!is_check("ls src"));
    assert!(!is_check("cat Cargo.toml && grep cargo README.md"));
    assert!(!is_check(""));
}
