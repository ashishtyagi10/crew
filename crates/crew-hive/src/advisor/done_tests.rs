//! What the turn's writes and checks make due.
use super::*;

fn call(tool: &str, args: &str) -> ToolCall {
    ToolCall {
        server: "sys".into(),
        tool: tool.into(),
        args: args.into(),
    }
}

#[test]
fn only_an_applied_write_with_no_passing_check_after_it_is_due() {
    let mut d = Done::default();
    d.call(&call("read_file", r#"{"path": "a.rs"}"#), true);
    assert!(!d.due(), "a read writes nothing");
    d.call(&call("edit", r#"{"path": "a.rs"}"#), false);
    assert!(!d.due(), "a failed edit wrote nothing");
    d.call(&call("write_file", r#"{"path": "a.rs"}"#), true);
    assert!(d.due());
    d.call(&call("run", r#"{"cmd": "cd a && cargo test"}"#), false);
    assert!(d.due(), "a check that failed proves nothing");
    d.call(&call("run", r#"{"cmd": "ls"}"#), true);
    assert!(d.due(), "ls is no check");
    d.call(&call("run", r#"{"cmd": "cd a && cargo test"}"#), true);
    assert!(!d.due());
}

#[test]
fn heard_or_asked_since_the_write_is_not_due_until_the_next_write() {
    let mut d = Done::default();
    d.call(&call("edit", r#"{"path": "a.rs"}"#), true);
    d.heard = true;
    assert!(!d.due());
    d.call(&call("edit", r#"{"path": "a.rs"}"#), true);
    assert!(d.due(), "a new write is news");
    d.asked = true;
    d.call(&call("edit", r#"{"path": "b.rs"}"#), true);
    assert!(!d.due(), "once a turn");
}

#[test]
fn prose_writes_are_never_due_and_leave_a_code_write_as_it_was() {
    let mut d = Done::default();
    for p in [
        "README.md",
        "notes.TXT",
        "Cargo.lock",
        "./docs/a.rs",
        "/r/.crew/plan.rs",
    ] {
        d.call(&call("write_file", &format!(r#"{{"path": "{p}"}}"#)), true);
    }
    assert!(!d.due(), "prose only");
    d.call(&call("edit", r#"{"path": "src/a.rs"}"#), true);
    assert!(d.due(), "prose and code: the code is unchecked");
    d.call(&call("run", r#"{"cmd": "cargo test"}"#), true);
    d.call(&call("edit", r#"{"path": "CHANGELOG.md"}"#), true);
    assert!(!d.due(), "a note after a passing check needs none");
}

/// A surface that runs nothing.
struct Quiet;

impl crate::tools::Tools for Quiet {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, _: &str, _: &str, _: &str) -> Result<String, String> {
        Ok(String::new())
    }
}

#[test]
fn a_repo_check_that_runs_after_the_task_skips_the_read() {
    let advisor = |after: bool| {
        let ask: super::super::Ask = Arc::new(|_| Box::pin(async { Err(String::new()) }));
        let hooks = super::super::Hooks {
            say: Arc::new(|_| {}),
            spent: Arc::new(|_| {}),
        };
        let find = Arc::new(move || Some(Arc::clone(&ask)));
        let a = super::super::Advisor::new("fix it", Arc::new(Quiet), find, hooks)
            .with_check_after(Arc::new(move || after));
        a.state()
            .done
            .call(&call("edit", r#"{"path": "src/a.rs"}"#), true);
        a
    };
    assert!(advisor(false).before_done("done").is_some());
    assert!(advisor(true).before_done("done").is_none());
}
