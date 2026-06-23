use super::*;

#[test]
fn relay_with_inline_message() {
    let r = parse_routing("TO codex: please review this");
    assert_eq!(
        r,
        Routing::Relay {
            to: "codex".into(),
            body: "please review this".into()
        }
    );
}

#[test]
fn relay_is_case_insensitive_and_keeps_peer_case() {
    let r = parse_routing("to Codex: hi");
    assert_eq!(
        r,
        Routing::Relay {
            to: "Codex".into(),
            body: "hi".into()
        }
    );
}

#[test]
fn relay_takes_following_lines_as_body() {
    let r = parse_routing("TO opencode:\nline one\nline two");
    assert_eq!(
        r,
        Routing::Relay {
            to: "opencode".into(),
            body: "line one\nline two".into()
        }
    );
}

#[test]
fn done_bare_and_with_answer() {
    assert_eq!(parse_routing("DONE"), Routing::Done(String::new()));
    assert_eq!(
        parse_routing("done: all set"),
        Routing::Done("all set".into())
    );
}

#[test]
fn plain_reply_passes_through() {
    assert_eq!(
        parse_routing("  the answer is 42 "),
        Routing::Reply("the answer is 42".into())
    );
}

#[test]
fn frame_mentions_self_peers_and_body() {
    let env = Envelope::new("user", "claude", "t", "build a thing");
    let p = frame(&env, &["codex".into(), "opencode".into()]);
    assert!(p.contains("\"claude\""));
    assert!(p.contains("codex, opencode"));
    assert!(p.contains("build a thing"));
    assert!(p.contains("TO <peer>"));
    assert!(p.contains("DONE"));
}

#[test]
fn frame_handles_no_peers() {
    let env = Envelope::new("user", "claude", "t", "hi");
    assert!(frame(&env, &[]).contains("(none)"));
}
