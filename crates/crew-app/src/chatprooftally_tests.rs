//! The verdict carries the run's own count, and a run of no tests is not a
//! check: `cargo test <filter>` that matched nothing exits 0.
use crew_hive::{AgentId, HiveEvent};
use crew_plugin::PluginEvent;

use super::tests::{call, settled};

const EDIT: &str = r#"{"file_path":"lib.rs"}"#;
const TEST: &str = r#"{"command":"cargo test -p crew-theme"}"#;

fn ran(ok: bool, out: &str, ms: u64) -> PluginEvent {
    PluginEvent::Hive {
        event: HiveEvent::ToolResult {
            agent: AgentId::minted("claude"),
            label: "Bash".into(),
            ok,
            text: out.into(),
            ms,
        },
    }
}

fn after_an_edit(ok: bool, out: &str, ms: u64) -> String {
    settled(&[call("Edit", EDIT), call("Bash", TEST), ran(ok, out, ms)]).unwrap()
}

#[test]
fn a_green_run_says_how_many_tests_and_how_long() {
    let out = "running 42 tests\ntest result: ok. 42 passed; 0 failed; 0 ignored\n";
    assert_eq!(
        after_an_edit(true, out, 12_400),
        "\u{2713} checked: `cargo test -p crew-theme` \u{2014} 42 tests passed in 12s after the last edit"
    );
}

#[test]
fn a_run_that_matched_no_test_is_not_checked() {
    let out = "running 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; \
               42 filtered out\n";
    assert_eq!(
        after_an_edit(true, out, 300),
        "! not checked: `cargo test -p crew-theme` ran no tests after the last edit"
    );
}

#[test]
fn a_red_run_names_what_failed() {
    let out = "test a::one ... FAILED\ntest a::two ... ok\ntest b::three ... FAILED\n\
               test c::four ... FAILED\ntest result: FAILED. 41 passed; 3 failed; 0 ignored\n";
    assert_eq!(
        after_an_edit(false, out, 3_000),
        "\u{2717} failing: `cargo test -p crew-theme` \u{2014} 3 of 44 tests failed after the last edit: one, three +1"
    );
}

/// A build, or output the runners' formats do not cover: the old words.
#[test]
fn a_run_with_no_count_reads_as_before() {
    assert_eq!(
        after_an_edit(true, "Finished `test` profile", 9_000),
        "\u{2713} checked: `cargo test -p crew-theme` passed after the last edit"
    );
}
