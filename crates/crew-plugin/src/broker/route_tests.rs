use super::*;

#[test]
fn next_directive_relays_with_body_above_it() {
    let r = parse_routing("Here is my review.\nLooks solid.\n@next codex");
    assert_eq!(
        r,
        Routing::Relay {
            to: "codex".into(),
            body: "Here is my review.\nLooks solid.".into()
        }
    );
}

#[test]
fn next_is_case_insensitive_and_keeps_peer_case() {
    let r = parse_routing("ok\n@NEXT Codex");
    assert_eq!(
        r,
        Routing::Relay {
            to: "Codex".into(),
            body: "ok".into()
        }
    );
}

#[test]
fn next_tolerates_colon_and_trailing_words() {
    let r = parse_routing("do it\n@next: opencode please");
    assert_eq!(
        r,
        Routing::Relay {
            to: "opencode".into(),
            body: "do it".into()
        }
    );
}

#[test]
fn done_ends_with_body_above() {
    assert_eq!(
        parse_routing("the answer is 42\n@done"),
        Routing::Done("the answer is 42".into())
    );
    assert_eq!(parse_routing("@done"), Routing::Done(String::new()));
}

#[test]
fn trailing_blank_lines_are_ignored() {
    assert_eq!(
        parse_routing("answer\n@done\n\n  \n"),
        Routing::Done("answer".into())
    );
}

#[test]
fn missing_directive_ends_thread_without_misrouting() {
    // No control line: don't guess a recipient — finish with the whole reply.
    assert_eq!(
        parse_routing("just some prose with no directive"),
        Routing::Done("just some prose with no directive".into())
    );
}

#[test]
fn frame_includes_task_transcript_peers_and_protocol() {
    let env = Envelope::new("codex", "claude", "t", "please review");
    let p = frame(
        &env,
        &["codex".into(), "opencode".into()],
        "build a parser",
        "user → claude: start\nclaude → codex: drafted",
    );
    assert!(p.contains("\"claude\""));
    assert!(p.contains("codex, opencode"));
    assert!(p.contains("build a parser")); // the task
    assert!(p.contains("claude → codex: drafted")); // the transcript
    assert!(p.contains("please review")); // the current message
    assert!(p.contains("@next") && p.contains("@done"));
}

#[test]
fn clip_flattens_short_text_unchanged() {
    assert_eq!(clip("hello\n  world", 100), "hello world");
}

#[test]
fn clip_truncates_long_text_with_ellipsis() {
    let out = clip(&"x ".repeat(500), 10);
    assert_eq!(out.chars().count(), 11); // 10 chars + the ellipsis
    assert!(out.ends_with('…'));
}

#[test]
fn frame_handles_no_peers_and_empty_transcript() {
    let env = Envelope::new("user", "claude", "t", "hi");
    let p = frame(&env, &[], "task", "");
    assert!(p.contains("(none)"));
    assert!(p.contains("you are first"));
}
