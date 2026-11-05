//! Red before the fix, green after: the verdict says the fix was proven.
use crew_hive::{AgentId, HiveEvent};
use crew_plugin::PluginEvent;

use crate::chatproof::Proof;

const EDIT: &str = r#"{"file_path":"lib.rs"}"#;
const TEST: &str = r#"{"command":"cargo test -p crew-theme"}"#;
const ONE: &str = r#"{"command":"cargo test -p crew-theme swap_rgb"}"#;
const RED: &str = "running 44 tests\ntest swap_rgb ... FAILED\n\
                   test result: FAILED. 42 passed; 2 failed; 0 ignored\n";
const GREEN: &str = "running 44 tests\ntest result: ok. 44 passed; 0 failed; 0 ignored\n";

fn call(label: &str, args: &str) -> PluginEvent {
    PluginEvent::Hive {
        event: HiveEvent::ToolCall {
            agent: AgentId::minted("claude"),
            label: label.into(),
            args: args.into(),
        },
    }
}

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

fn settled(events: &[PluginEvent]) -> String {
    let mut p = Proof::default();
    events.iter().for_each(|e| p.hear(e));
    p.settle().unwrap()
}

fn edit() -> PluginEvent {
    call("Edit", EDIT)
}

#[test]
fn fail_then_edit_then_pass_names_both_runs() {
    let said = settled(&[
        call("Bash", TEST),
        ran(false, RED, 9_000),
        edit(),
        call("Bash", TEST),
        ran(true, GREEN, 12_400),
    ]);
    assert_eq!(
        said,
        "\u{2713} fixed: `cargo test -p crew-theme` \u{2014} 2 of 44 failed before the fix; \
         44 pass (12s)"
    );
}

#[test]
fn a_wider_run_after_covers_the_narrow_reproduction() {
    let said = settled(&[
        call("Bash", ONE),
        ran(false, "", 900),
        edit(),
        call("Bash", TEST),
        ran(true, GREEN, 400),
    ]);
    assert_eq!(
        said,
        "\u{2713} fixed: `cargo test -p crew-theme swap_rgb` failed before the fix; \
         `cargo test -p crew-theme` passes (44 tests)"
    );
}

#[test]
fn a_turn_that_never_reproduced_is_only_checked() {
    let said = settled(&[edit(), call("Bash", TEST), ran(true, GREEN, 400)]);
    assert!(said.starts_with("\u{2713} checked:"), "{said}");
}

#[test]
fn an_unrelated_red_run_proves_nothing_about_the_fix() {
    let lint = r#"{"command":"cargo clippy"}"#;
    let said = settled(&[
        call("Bash", lint),
        ran(false, "", 900),
        edit(),
        call("Bash", TEST),
        ran(true, GREEN, 400),
    ]);
    assert!(said.starts_with("\u{2713} checked:"), "{said}");
}

#[test]
fn a_flaky_red_that_passed_again_before_the_edit_is_no_reproduction() {
    let said = settled(&[
        call("Bash", TEST),
        ran(false, RED, 900),
        call("Bash", TEST),
        ran(true, GREEN, 900),
        edit(),
        call("Bash", TEST),
        ran(true, GREEN, 400),
    ]);
    assert!(said.starts_with("\u{2713} checked:"), "{said}");
}

#[test]
fn still_red_after_the_edit_is_failing_not_fixed() {
    let said = settled(&[
        call("Bash", TEST),
        ran(false, RED, 900),
        edit(),
        call("Bash", TEST),
        ran(false, RED, 400),
    ]);
    assert!(said.starts_with("\u{2717} failing:"), "{said}");
}
