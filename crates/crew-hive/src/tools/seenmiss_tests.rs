use super::*;

/// `fs:read` only looks; `sys:edit` and `sys:run` may write.
struct Surface;

impl Tools for Surface {
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

const EDIT: &str = r#"{"path": "a.rs", "old": "let x = 1;", "new": "let x = 2;"}"#;

fn failed(seen: &mut Seen, c: &ToolCall, round: u32) {
    seen.ran(&Surface, c, round, false);
    seen.miss(
        c,
        round,
        "ERROR: edit a.rs: `old` is not in the file \u{2014} the nearest is line 2",
    );
}

#[test]
fn a_failed_edit_sent_again_is_known_to_fail() {
    let mut seen = Seen::default();
    let edit = call("sys:edit", EDIT);
    assert_eq!(seen.missed(&edit), None);
    failed(&mut seen, &edit, 3);
    let (round, why) = seen.missed(&edit).expect("known");
    assert_eq!(round, 3);
    let said = Seen::again(round, why);
    assert!(
        said.starts_with("ERROR: the same edit failed in round 3"),
        "{said}"
    );
    assert!(said.contains("`old` is not in the file"), "{said}");
    // The same call spelled differently is the same call.
    let respaced = call(
        "sys:edit",
        r#"{"new":"let x = 2;","old":"let x = 1;","path":"a.rs"}"#,
    );
    assert!(seen.missed(&respaced).is_some());
}

/// Anything written since, or a command that may have, and the edit is
/// tried again; a read, or another edit that failed, changes nothing.
#[test]
fn a_write_since_lets_it_run_again() {
    let edit = call("sys:edit", EDIT);
    for (between, ok, known) in [
        (call("fs:read", r#"{"path":"a.rs"}"#), true, true),
        (
            call("sys:edit", r#"{"path":"b.rs","old":"x","new":"y"}"#),
            false,
            true,
        ),
        (
            call("sys:edit", r#"{"path":"b.rs","old":"x","new":"y"}"#),
            true,
            false,
        ),
        (call("sys:run", r#"{"cmd":"cargo fmt"}"#), false, false),
    ] {
        let mut seen = Seen::default();
        failed(&mut seen, &edit, 1);
        seen.ran(&Surface, &between, 2, ok);
        assert_eq!(seen.missed(&edit).is_some(), known, "{between:?} ok={ok}");
    }
}

/// Only an edit is remembered: a failed command may have written halfway.
#[test]
fn a_failed_command_is_not_remembered() {
    let mut seen = Seen::default();
    let run = call("sys:run", r#"{"cmd":"cargo test"}"#);
    seen.ran(&Surface, &run, 1, false);
    seen.miss(&run, 1, "exit 101");
    assert_eq!(seen.missed(&run), None);
}

/// The rows the first refusal showed, the file's lines to copy, are in the
/// answer to the repeat: a14x2 was given only the first line, four times.
#[test]
fn the_repeat_keeps_the_rows_the_first_failure_showed() {
    let why = "ERROR: edit lib.rs: `old` is not in the file: its first 4 lines are lines \
               165\u{2013}168, then it parts \u{2014} where `old` has \u{201c}(hi + 0.05)\u{201d}, \
               line 169 reads (copy it, not the numbers):\n\
               169\u{2502}     ((hi + 0.05) / (lo + 0.05)) as f64\n170\u{2502} }";
    let said = Seen::again(4, why);
    assert!(
        said.contains("line 169 reads (copy it, not the numbers): \u{2014} change `old`"),
        "{said}"
    );
    assert!(
        said.ends_with("\n169\u{2502}     ((hi + 0.05) / (lo + 0.05)) as f64\n170\u{2502} }"),
        "{said}"
    );
}
