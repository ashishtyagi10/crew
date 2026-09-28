//! The first hop's prompt says the task once.
use super::*;

#[test]
fn the_first_hop_says_its_message_once_and_keeps_its_tools() {
    let body = "PROJECT (…): a GPU terminal\n\nwhat does crew-render do?";
    let env = Envelope::new("user", "coder", "t", body);
    let p = frame(
        &env,
        Some(""),
        &["reviewer (review, critique)".into()],
        body,
        "TOOLS: sys:read_file",
        "",
    );
    assert_eq!(p.matches("what does crew-render do?").count(), 1, "{p}");
    assert!(p.contains("TOOLS: sys:read_file"), "{p}");
    assert!(
        p.contains("(the message from \"user\" at the end of this prompt)"),
        "{p}"
    );
}

#[test]
fn a_later_hop_restates_the_task_because_the_message_is_a_peers() {
    let env = Envelope::new("coder", "reviewer", "t", "here is my draft");
    let p = frame(&env, Some(""), &[], "write the release notes", "", "");
    assert!(p.contains("write the release notes"), "{p}");
    assert!(p.contains("here is my draft"), "{p}");
}

#[test]
fn roles_are_said_once_and_the_hand_off_list_is_names() {
    let env = Envelope::new("user", "coder", "t", "go");
    let peers = vec![
        "reviewer (review, critique)".to_string(),
        "planner (planning)".to_string(),
    ];
    let p = frame(&env, Some(""), &peers, "go", "", "");
    assert_eq!(p.matches("(review, critique)").count(), 1, "{p}");
    assert!(p.contains("(only from: reviewer, planner)"), "{p}");
}
