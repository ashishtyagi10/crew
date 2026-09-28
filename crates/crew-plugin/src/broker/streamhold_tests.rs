use super::*;

/// Feed `frags` one by one and return everything shown.
fn shown(frags: &[&str]) -> String {
    let mut h = Hold::default();
    frags.iter().map(|f| h.feed(f)).collect()
}

#[test]
fn a_trailing_done_never_shows_even_split_across_fragments() {
    assert_eq!(shown(&["2+2 is 4.\n\n@", "do", "ne"]), "2+2 is 4.\n\n");
    assert_eq!(shown(&["answer\n@done\n"]), "answer\n");
}

#[test]
fn a_tool_directive_and_its_json_are_dropped_and_the_next_round_is_clean() {
    let s = shown(&[
        "Let me read it.\n@tool sys:read_file {\"path\": \"READ",
        "ME.md\", \"offset\": 0}\n",
        "- The crate draws the window.\n",
    ]);
    assert_eq!(s, "Let me read it.\n- The crate draws the window.\n");
}

#[test]
fn a_hand_off_is_dropped_but_a_mention_at_line_start_is_released_whole() {
    assert_eq!(shown(&["done here\n@next writer\n"]), "done here\n");
    assert_eq!(
        shown(&["@editor, over to you:", " tighten it\nok"]),
        "@editor, over to you: tighten it\nok"
    );
}

#[test]
fn wrapped_directives_are_still_directives() {
    assert_eq!(shown(&["x\n`@done`\n"]), "x\n");
    assert_eq!(shown(&["x\n  **@next coder**\n"]), "x\n");
    // Only a whole word: `@doneness` is prose.
    assert_eq!(shown(&["@doneness matters\n"]), "@doneness matters\n");
}

#[test]
fn text_without_directives_passes_untouched_and_undelayed() {
    let mut h = Hold::default();
    assert_eq!(h.feed("The `crew-render` crate"), "The `crew-render` crate");
    assert_eq!(
        h.feed(" uses *wgpu* @ 60fps\n  - item"),
        " uses *wgpu* @ 60fps\n  - item"
    );
}

#[test]
fn a_fenced_call_leaves_no_empty_block_behind() {
    let s = shown(&[
        "Let me check.\n``",
        "`\n@tool sys:read_file {\"path\": \"x\"}\n```\n",
        "Found it.\n",
    ]);
    assert_eq!(s, "Let me check.\nFound it.\n");
}

#[test]
fn pretty_printed_json_under_a_call_never_shows() {
    let s = shown(&[
        "@tool sys:edit {\n  \"path\": \"a.rs\",\n",
        "  \"old\": \"}\",\n  \"new\": \"y\"\n}**\n",
        "Edited.\n",
    ]);
    assert_eq!(s, "Edited.\n");
    // JSON opened on the line under a bare call, inside a json fence.
    let s = shown(&["```json\n@tool sys:read_file\n  {\n  \"path\": \"x\"\n}\n```\nRead.\n"]);
    assert_eq!(s, "Read.\n");
}

#[test]
fn a_code_block_that_wraps_no_call_is_shown_as_written() {
    let text = "Run:\n```sh\ncargo test\n@dataclass\n```\nThen look.\n```\n```\n";
    assert_eq!(shown(&[text]), text);
    // A call right after a block does not take the block's closer for its own.
    let s = shown(&["```\nx\n```\n@tool sys:run {}\n", "```\ny\n```\n"]);
    assert_eq!(s, "```\nx\n```\n```\ny\n```\n");
    // Nor does a fenced call left unclosed eat the next round's block.
    let s = shown(&["```\n@tool a:b {}\n", "Here:\n```\ncode\n```\n"]);
    assert_eq!(s, "Here:\n```\ncode\n```\n");
    // A bare call followed by prose: nothing swallowed but the indent.
    assert_eq!(shown(&["@tool sys:list_dir\n", "Listed.\n"]), "Listed.\n");
}
