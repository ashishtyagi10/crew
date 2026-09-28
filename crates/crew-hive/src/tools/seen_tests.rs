use super::*;

/// A surface where `fs:read` only looks and everything else may write.
struct Reads;

impl Tools for Reads {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        unreachable!("Seen never runs a tool")
    }
    fn repeatable(&self, server: &str, tool: &str) -> bool {
        (server, tool) == ("fs", "read")
    }
}

fn call(label: &str, args: &str) -> ToolCall {
    let (server, tool) = label.split_once(':').unwrap();
    ToolCall {
        server: server.into(),
        tool: tool.into(),
        args: args.into(),
    }
}

#[test]
fn a_read_that_succeeded_is_a_repeat_the_second_time() {
    let mut seen = Seen::default();
    let read = call("fs:read", r#"{"path":"a.rs"}"#);
    assert_eq!(
        seen.check(&read),
        None,
        "nothing is a repeat the first time"
    );
    seen.ran(&Reads, &read, 2, true);
    assert_eq!(seen.check(&read), Some(2));
    // Run again (its first result was shortened out of the prompt), the NEW
    // round is the one pointed at: that is where the result is whole.
    seen.ran(&Reads, &read, 5, true);
    assert_eq!(seen.check(&read), Some(5));
}

#[test]
fn spacing_and_key_order_do_not_make_a_new_call() {
    let mut seen = Seen::default();
    seen.ran(
        &Reads,
        &call(
            "fs:read",
            r#"{"path": "a.rs", "opts": {"b": 1, "a": [2, {"y":1,"x":0}]}}"#,
        ),
        1,
        true,
    );
    let same = call(
        "fs:read",
        "  {\"opts\":{\"a\":[2,{\"x\":0,\"y\":1}],\"b\":1},\"path\":\"a.rs\"}\n",
    );
    assert_eq!(seen.check(&same), Some(1));
    assert_eq!(seen.check(&call("fs:read", r#"{"path":"b.rs"}"#)), None);
}

#[test]
fn arguments_that_are_not_json_compare_as_written_trimmed() {
    let mut seen = Seen::default();
    seen.ran(&Reads, &call("fs:read", "a.rs"), 1, true);
    assert_eq!(seen.check(&call("fs:read", "  a.rs\n")), Some(1));
    assert_eq!(seen.check(&call("fs:read", "b.rs")), None);
}

#[test]
fn a_write_forgets_every_read_even_one_that_failed() {
    let mut seen = Seen::default();
    let read = call("fs:read", r#"{"path":"a.rs"}"#);
    seen.ran(&Reads, &read, 1, true);
    seen.ran(&Reads, &call("fs:write", r#"{"path":"a.rs"}"#), 2, false);
    assert_eq!(seen.check(&read), None, "the file may have changed");
}

#[test]
fn a_failed_read_is_never_remembered() {
    let mut seen = Seen::default();
    let read = call("fs:read", r#"{"path":"a.rs"}"#);
    seen.ran(&Reads, &read, 1, false);
    assert_eq!(seen.check(&read), None, "retrying a failure can be right");
}

#[test]
fn a_tool_the_surface_never_classified_is_never_skipped() {
    struct Silent;
    impl Tools for Silent {
        fn hint(&self) -> String {
            String::new()
        }
        fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
            unreachable!()
        }
    }
    let mut seen = Seen::default();
    let read = call("fs:read", r#"{"path":"a.rs"}"#);
    seen.ran(&Silent, &read, 1, true);
    assert_eq!(seen.check(&read), None);
}

#[test]
fn the_pointer_names_the_round_and_is_one_line() {
    let p = Seen::pointer(3);
    assert!(p.contains("round 3") && p.contains("above"), "{p}");
    assert_eq!(p.lines().count(), 1);
}
