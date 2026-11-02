use crew_hive::{AgentId, HiveEvent};
use crew_plugin::PluginEvent;

use super::{effects, Effect};
use crate::chatproof::Proof;

fn call(label: &str, args: serde_json::Value) -> PluginEvent {
    let (agent, label, args) = (AgentId::minted("claude"), label.into(), args.to_string());
    PluginEvent::Hive {
        event: HiveEvent::ToolCall { agent, label, args },
    }
}

fn bash(cmd: &str) -> PluginEvent {
    call("Bash", serde_json::json!({ "command": cmd }))
}

fn result(ok: bool, text: &str) -> PluginEvent {
    PluginEvent::Hive {
        event: HiveEvent::ToolResult {
            agent: AgentId::minted("claude"),
            label: "Bash".into(),
            ok,
            text: text.into(),
            ms: 700,
        },
    }
}

fn edit() -> PluginEvent {
    call("Edit", serde_json::json!({ "file_path": "lib.rs" }))
}

fn settled(events: &[PluginEvent]) -> String {
    let mut p = Proof::default();
    events.iter().for_each(|e| p.hear(e));
    p.settle().unwrap_or_default()
}

#[test]
fn a_file_named_like_a_test_is_not_a_test_run() {
    for cmd in [
        "wc -l src/chatproof_tests.rs",
        "git diff -- src/x_tests.rs",
        "ls tests/",
        "cat build.rs",
        "grep -n check src/lib.rs",
    ] {
        assert_eq!(effects(cmd), vec![], "{cmd}");
        let said = settled(&[edit(), bash(cmd), result(true, "")]);
        assert!(said.starts_with("! not checked"), "{cmd}: {said}");
    }
}

#[test]
fn real_runs_still_count_behind_cd_env_and_wrappers() {
    for cmd in [
        "cargo test -p crew-theme",
        "cd crates/x && cargo test -p y",
        "npm run test",
        "pnpm test:unit",
        "CI=1 pytest -q",
        "python3 -m pytest tests",
        "time npx jest",
        "go vet ./...",
        "./scripts/run-tests.sh",
        "cargo fmt --check",
        "cargo test 2>&1 | tail -20",
        "cargo test > /tmp/t.log 2>&1",
    ] {
        assert_eq!(effects(cmd), vec![Effect::Verify], "{cmd}");
    }
}

#[test]
fn a_later_green_lint_does_not_hide_a_red_test() {
    let said = settled(&[
        edit(),
        bash("cargo test"),
        result(false, ""),
        bash("cargo fmt --check"),
        result(true, ""),
    ]);
    assert!(said.starts_with("\u{2717} failing: `cargo test`"), "{said}");
    let again = settled(&[
        edit(),
        bash("cargo test"),
        result(false, ""),
        bash("cargo test"),
        result(true, ""),
    ]);
    assert!(again.starts_with("\u{2713} checked"), "{again}");
}

#[test]
fn a_run_sent_to_the_background_has_not_passed() {
    let bg = serde_json::json!({ "command": "cargo test", "run_in_background": true });
    let said = settled(&[
        edit(),
        call("Bash", bg),
        result(true, "Command running in background with ID: b1"),
        bash("sleep 5"),
        result(true, ""),
    ]);
    assert!(said.starts_with("? `cargo test`"), "{said}");
}

#[test]
fn edits_made_through_the_shell_count_as_edits() {
    for cmd in [
        "sed -i '' s/a/b/ src/x.rs",
        "perl -pi -e s/a/b/ x.rs",
        "cargo fmt",
        "git apply fix.patch",
        "git checkout -- src/x.rs",
        "echo hi > src/x.rs",
        "cat >> notes.md",
        "printf x | tee src/x.rs",
    ] {
        assert_eq!(effects(cmd), vec![Effect::Write], "{cmd}");
    }
    let only_sed = settled(&[bash("sed -i '' s/a/b/ src/x.rs"), result(true, "")]);
    assert!(only_sed.starts_with("! not checked"), "{only_sed}");
    let fmt_after = settled(&[
        edit(),
        bash("cargo test"),
        result(true, ""),
        bash("cargo fmt"),
        result(true, ""),
    ]);
    assert!(fmt_after.starts_with("! not checked"), "{fmt_after}");
    let sed_then_test = settled(&[
        bash("sed -i s/a/b/ x.rs"),
        bash("cargo test"),
        result(true, ""),
    ]);
    assert!(
        sed_then_test.starts_with("\u{2713} checked"),
        "{sed_then_test}"
    );
}

#[test]
fn scratch_writes_are_not_edits_and_order_holds_within_a_command() {
    for cmd in [
        "grep -n x f > /dev/null",
        "ls 2>&1",
        "cargo test | tee /tmp/log",
    ] {
        assert!(!effects(cmd).contains(&Effect::Write), "{cmd}");
    }
    assert_eq!(
        effects("cargo fmt && cargo test"),
        vec![Effect::Write, Effect::Verify]
    );
    let said = settled(&[edit(), bash("cargo test && cargo fmt"), result(true, "")]);
    assert!(said.starts_with("! not checked"), "{said}");
}
