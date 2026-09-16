use super::*;

#[test]
fn script_and_style_bodies_are_dropped_whole_not_rendered() {
    let html = "<html><head><title>t</title></head><body>\
        <style>.a{color:red}</style><script>var x = 1 < 2;</script>\
        <p>The actual sentence.</p></body></html>";
    let text = readable(html);
    assert!(text.contains("The actual sentence."), "{text}");
    assert!(!text.contains("color:red"), "{text}");
    assert!(!text.contains("var x"), "{text}");
    assert!(!text.contains("<"), "markup survived: {text}");
}

#[test]
fn a_block_tag_starts_a_line_and_an_inline_one_does_not() {
    let text = readable("<p>one</p><p>two <b>bold</b> tail</p>");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines, vec!["one", "two bold tail"], "{text}");
}

#[test]
fn the_entities_that_appear_in_prose_are_decoded() {
    assert_eq!(
        readable("<p>a&nbsp;b &amp; c &lt;d&gt; &#39;e&#39;</p>"),
        "a b & c <d> 'e'"
    );
    // The hex apostrophe is what search results are titled with.
    assert_eq!(readable("<p>You&#x27;ll see</p>"), "You'll see");
    // Anything else is left alone rather than guessed at.
    assert_eq!(readable("<p>&copy; 2026</p>"), "&copy; 2026");
}

#[test]
fn whitespace_collapses_and_never_leaves_more_than_one_blank_line() {
    // Runs of spaces inside a line become one; a run of empty blocks (and a
    // run of blank source lines) becomes at most one blank line.
    assert_eq!(readable("<p>a     b   c</p>"), "a b c");
    let text = readable("<div>a\n\n\n\nb</div><div></div><div></div><div>c</div>");
    assert_eq!(text, "a\n\nb\n\nc", "{text:?}");
}

#[test]
fn an_unclosed_script_takes_the_rest_of_the_document_rather_than_leaking_it() {
    let text = readable("<p>before</p><script>secret = 1;");
    assert_eq!(text, "before");
}

#[test]
fn a_long_page_is_cut_with_a_marker_that_says_how_much_was_left() {
    let out = capped(&"x".repeat(100), 10);
    assert!(out.starts_with(&"x".repeat(10)));
    assert!(out.contains("clipped 90 chars of 100"), "{out}");
    assert_eq!(capped("short", 10), "short");
}
