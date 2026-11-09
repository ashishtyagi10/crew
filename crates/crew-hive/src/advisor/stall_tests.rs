//! What counts as a round that told the agent nothing new.
use super::*;

fn read(path: &str) -> ToolCall {
    ToolCall {
        server: "sys".into(),
        tool: "read_file".into(),
        args: format!("{{\"path\": \"{path}\"}}"),
    }
}

fn ok(s: &str) -> (bool, String) {
    (true, s.to_string())
}

#[test]
fn a_repeat_with_the_same_result_is_a_stall_asked_about_once() {
    let mut p = Progress::default();
    assert!(!p.round(&[read("a.rs")], &[ok("A")]), "first read is new");
    // Spacing and key order do not make a call new (`seen::key`).
    let respaced = ToolCall {
        args: "{\"path\":\"a.rs\"}".into(),
        ..read("a.rs")
    };
    assert!(p.round(&[respaced], &[ok("A")]), "same call, same text");
    assert!(!p.round(&[read("a.rs")], &[ok("A")]), "already asked");
    assert!(!p.round(&[read("b.rs")], &[ok("B")]), "learned something");
    assert!(p.round(&[read("b.rs")], &[ok("B")]), "a new stall is asked");
}

#[test]
fn seens_pointer_is_a_repeat_and_a_changed_result_is_not() {
    let mut p = Progress::default();
    p.round(&[read("a.rs")], &[ok("A")]);
    let pointer = Seen::pointer(1);
    assert!(p.round(&[read("a.rs")], &[ok(&pointer)]));
    let mut p = Progress::default();
    p.round(&[read("a.rs")], &[ok("A")]);
    assert!(!p.round(&[read("a.rs")], &[ok("A, edited")]), "new text");
}

#[test]
fn one_new_call_in_the_round_or_a_checklist_alone_is_progress() {
    let mut p = Progress::default();
    p.round(&[read("a.rs")], &[ok("A")]);
    let both = [read("a.rs"), read("b.rs")];
    assert!(!p.round(&both, &[ok("A"), ok("B")]));
    let tick = ToolCall {
        server: "sys".into(),
        tool: "todo".into(),
        args: "{}".into(),
    };
    assert!(!p.round(&[tick.clone()], &[ok("list")]), "nothing counted");
    assert!(p.round(&[tick, read("a.rs")], &[ok("list"), ok("A")]));
}

#[test]
fn the_same_error_twice_is_a_stall() {
    let mut p = Progress::default();
    let edit = ToolCall {
        server: "sys".into(),
        tool: "edit".into(),
        args: "{\"path\": \"a.rs\"}".into(),
    };
    let miss = (false, "ERROR: old text not found".to_string());
    assert!(!p.round(std::slice::from_ref(&edit), std::slice::from_ref(&miss)));
    let again = (false, Seen::again(1, "old text not found"));
    assert!(p.round(&[edit], &[again]));
}
