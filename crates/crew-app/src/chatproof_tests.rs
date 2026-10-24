use crew_hive::{AgentId, HiveEvent};
use crew_plugin::PluginEvent;

use super::Proof;

fn call(label: &str, args: &str) -> PluginEvent {
    PluginEvent::Hive {
        event: HiveEvent::ToolCall {
            agent: AgentId::minted("claude"),
            label: label.into(),
            args: args.into(),
        },
    }
}

fn result(label: &str, ok: bool) -> PluginEvent {
    PluginEvent::Hive {
        event: HiveEvent::ToolResult {
            agent: AgentId::minted("claude"),
            label: label.into(),
            ok,
            text: String::new(),
            ms: 700,
        },
    }
}

fn card(text: &str) -> PluginEvent {
    PluginEvent::Message {
        channel: "crew".into(),
        sender: "engineer".into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
    }
}

fn settled(events: &[PluginEvent]) -> Option<String> {
    let mut p = Proof::default();
    events.iter().for_each(|e| p.hear(e));
    p.settle()
}

const TEST: &str = r#"{"command":"cargo test -p crew-theme","description":"run"}"#;

#[test]
fn claude_codes_edit_then_a_green_test_run_is_checked() {
    let said = settled(&[
        call("Edit", r#"{"file_path":"lib.rs"}"#),
        call("Bash", TEST),
        result("Bash", true),
    ]);
    assert_eq!(
        said.as_deref(),
        Some("\u{2713} checked: `cargo test -p crew-theme` passed after the last edit")
    );
}

#[test]
fn a_red_run_after_the_edit_is_failing() {
    let said = settled(&[
        call("Write", "{}"),
        call("Bash", TEST),
        result("Bash", false),
    ]);
    assert!(said
        .unwrap()
        .starts_with("\u{2717} failing: `cargo test -p crew-theme`"));
}

#[test]
fn an_edit_after_a_green_run_is_not_checked_any_more() {
    let said = settled(&[
        call("Edit", "{}"),
        call("Bash", TEST),
        result("Bash", true),
        call("Edit", "{}"),
    ]);
    assert!(said.unwrap().starts_with("! not checked"));
}

#[test]
fn a_command_that_builds_or_tests_nothing_is_not_evidence() {
    let ls = r#"{"command":"ls -la crates"}"#;
    let said = settled(&[call("Edit", "{}"), call("Bash", ls), result("Bash", true)]);
    assert!(said.unwrap().starts_with("! not checked"));
}

#[test]
fn crews_own_tool_cards_are_heard_the_same_way() {
    let said = settled(&[
        card("[tool] sys:edit  crates/crew-theme/src/lib.rs"),
        card("[tool] sys:run  cargo test -p crew-theme"),
        card("[tool] sys:run \u{2713} 0.7s\nexit 0\n\nrunning 176 tests"),
    ]);
    assert_eq!(
        said.as_deref(),
        Some("\u{2713} checked: `cargo test -p crew-theme` passed after the last edit")
    );
    let red = settled(&[
        card("[tool] sys:write_file  a.rs"),
        card("[tool] sys:run  cargo build"),
        card("[tool] sys:run \u{2717} 0.8s\nexit 101"),
    ]);
    assert!(red.unwrap().starts_with("\u{2717} failing: `cargo build`"));
}

#[test]
fn a_turn_that_edited_nothing_or_ran_the_projects_check_says_nothing() {
    assert_eq!(settled(&[call("Bash", TEST), result("Bash", true)]), None);
    let checked = card("check: cargo test \u{2014} passed");
    assert_eq!(settled(&[call("Edit", "{}"), checked]), None);
}

#[test]
fn the_verdict_is_said_once_and_the_next_turn_starts_clean() {
    let mut p = Proof::default();
    p.hear(&call("Edit", "{}"));
    assert!(p.settle().is_some());
    assert_eq!(p.settle(), None);
}

#[test]
fn the_pane_says_it_under_the_turn_when_the_task_ends() {
    let mut p = crate::chat::tests::pane();
    p.absorb_task(3, true);
    for ev in [call("Edit", "{}"), call("Bash", TEST), result("Bash", true)] {
        p.heard(&ev);
    }
    p.absorb_task(3, false);
    let last = p.messages.last().expect("a note");
    assert_eq!(last.sender, "agent smith");
    assert!(last.text.starts_with("\u{2713} checked"), "{}", last.text);
}

/// crew's own tools on the hive lane (`hopturn::Pane`), as the S6 replay sent
/// them: an edit and a run in one reply, then both results.
#[test]
fn crews_tools_on_the_hive_lane_are_heard_too() {
    let said = settled(&[
        call("sys:edit", r#"{"path":"crates/crew-theme/src/lib.rs"}"#),
        call("sys:run", r#"{"cmd":"cargo test -p crew-theme"}"#),
        result("sys:edit", true),
        result("sys:run", true),
    ]);
    assert_eq!(
        said.as_deref(),
        Some("\u{2713} checked: `cargo test -p crew-theme` passed after the last edit")
    );
}
