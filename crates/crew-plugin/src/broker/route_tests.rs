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
fn tolerates_markdown_and_punctuation_wrappers() {
    // Real agents wrap directives; the parser must still recover them.
    assert_eq!(
        parse_routing("draft ready\n**@next codex**"),
        Routing::Relay {
            to: "codex".into(),
            body: "draft ready".into()
        }
    );
    assert_eq!(
        parse_routing("all set\n`@done`"),
        Routing::Done("all set".into())
    );
    assert_eq!(parse_routing("ok\n@done."), Routing::Done("ok".into()));
    assert_eq!(
        parse_routing("go\n@next `opencode`"),
        Routing::Relay {
            to: "opencode".into(),
            body: "go".into()
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
fn has_directive_detects_presence() {
    assert!(has_directive("answer\n@next codex"));
    assert!(has_directive("done\n**@done**"));
    assert!(!has_directive("just prose, no control line"));
    assert!(!has_directive(""));
}

#[test]
fn repair_prompt_shows_reply_and_asks_for_directive() {
    let p = repair_prompt(&["codex".into()], "my earlier reply");
    assert!(p.contains("my earlier reply"));
    assert!(p.contains("@next") && p.contains("@done"));
    assert!(p.contains("codex"));
}

#[test]
fn frame_orders_stable_prefix_before_variable_parts() {
    let env = Envelope::new("user", "claude", "t", "the message");
    let p = frame(&env, &["codex".into()], "the task", "user → claude: hi");
    let at = |s: &str| p.find(s).unwrap();
    // task + protocol (stable) precede transcript + message (variable).
    assert!(at("TASK:") < at("HOW TO REPLY"));
    assert!(at("HOW TO REPLY") < at("CONVERSATION SO FAR"));
    assert!(at("CONVERSATION SO FAR") < at("MESSAGE FOR YOU"));
}

#[test]
fn compact_ws_collapses_three_or_more_newlines_to_two() {
    assert_eq!(compact_ws("a\n\n\n\nb"), "a\n\nb");
    assert_eq!(compact_ws("a\n\n\n\n\n\nb"), "a\n\nb");
}

#[test]
fn compact_ws_strips_trailing_spaces_and_tabs_per_line() {
    assert_eq!(compact_ws("x   \ny"), "x\ny");
    assert_eq!(compact_ws("x\t\t\ny   \n"), "x\ny");
}

#[test]
fn compact_ws_leaves_already_compact_text_unchanged() {
    let s = "line one\nline two\n\nline three";
    assert_eq!(compact_ws(s), s);
}

#[test]
fn compact_ws_preserves_single_and_double_newlines() {
    // A single newline (no blank line) and a double newline (one blank line)
    // are both left untouched — only runs of 3+ collapse.
    assert_eq!(compact_ws("a\nb"), "a\nb");
    assert_eq!(compact_ws("a\n\nb"), "a\n\nb");
}

#[test]
fn frame_output_is_whitespace_compacted() {
    let env = Envelope::new("codex", "claude", "t", "hi   ");
    let p = frame(&env, &[], "task", "");
    assert_eq!(
        p,
        compact_ws(&p),
        "frame's return value must already be compact"
    );
}

#[test]
fn frame_handles_no_peers_and_empty_transcript() {
    let env = Envelope::new("user", "claude", "t", "hi");
    let p = frame(&env, &[], "task", "");
    assert!(p.contains("(none)"));
    assert!(p.contains("you are first"));
}
