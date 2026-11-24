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
    assert!(!is_check("npm install eslint"));
    assert!(!is_check("cargo fmt"));
    assert!(!is_check(""));
}

#[test]
fn a_reason_counts_as_a_whole_subcommand_not_a_substring() {
    assert!(!is_check("go get github.com/stretchr/testify"));
    assert!(!is_check("cargo run --bin build_x"));
    assert!(!is_check("npm run rebuild-docs"));
    assert!(is_check("yarn test:unit"));
    assert!(is_check("cargo +nightly test"));
}

#[test]
fn a_check_whose_exit_a_later_command_owns_is_not() {
    assert!(!is_check("cargo test; echo ok"));
    assert!(!is_check("cargo test || true"));
    assert!(!is_check("cargo test\necho ok"));
    assert!(!is_check("cargo test 2>&1 | tee log"));
    assert!(!is_check("cargo test | tail; echo"));
}

#[test]
fn a_check_cut_down_by_a_trailing_filter_is_still_one() {
    assert!(is_check("cargo test 2>&1 | tail -20"));
    assert!(is_check("cd a && cargo test 2>&1 | tail -5"));
    assert!(is_check(
        "cargo clippy --all-targets 2>&1 | grep -E error | head"
    ));
}

#[test]
fn lints_and_launched_runners_agents_type_are_checks() {
    for cmd in [
        "cargo clippy -- -D warnings",
        "go vet ./...",
        "cargo fmt --all --check",
        "ruff check .",
        "eslint src",
        "mypy app",
        "tsc --noEmit",
        "npx eslint .",
        "npx tsc -p .",
        "pnpm lint",
        "yarn tsc",
        "python -m pytest -x",
        "python3 -m pytest",
        "uv run pytest",
        "./gradlew test",
        "mvn verify",
        "bundle exec rspec",
    ] {
        assert!(is_check(cmd), "{cmd}");
    }
}
