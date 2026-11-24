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
