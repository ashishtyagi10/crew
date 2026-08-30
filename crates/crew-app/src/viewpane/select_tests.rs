//! Selecting, and the two commands that put a marker in a file whose markers
//! are never on screen.
use super::super::caret::Step;
use super::range;
use crate::viewpane::detect::Format;
use crate::viewpane::load::Loaded;
use crate::viewpane::{LoadState, ViewPane};

fn doc(text: &str) -> ViewPane {
    let mut p = ViewPane::open(std::env::temp_dir().join("select.md"));
    p.state = LoadState::Ready {
        format: Format::Markdown,
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    p.start_editing(60);
    p
}

fn text_of(p: &ViewPane) -> String {
    p.source_str().unwrap_or_default().to_string()
}

/// Select `n` characters from the caret.
fn select(p: &mut ViewPane, n: usize) {
    p.anchor_here();
    for _ in 0..n {
        p.move_caret(Step::Right, 60, 20);
    }
}

#[test]
fn nothing_is_selected_until_something_is() {
    let p = doc("hello world\n");
    assert_eq!(range(&p), None);
}

#[test]
fn a_shifted_move_selects_what_it_passes_over() {
    let mut p = doc("hello world\n");
    select(&mut p, 5);
    assert_eq!(p.selected_text().as_deref(), Some("hello"));
}

/// The SOURCE, not the render: what is copied out of a markdown document and
/// pasted into another one has to still be markdown.
#[test]
fn what_is_copied_is_the_markdown_not_the_rendering() {
    let mut p = doc("a **bold** word\n");
    p.anchor = Some(2);
    p.caret_at = Some(10);
    assert_eq!(p.selected_text().as_deref(), Some("**bold**"));
}

/// The promise is that no `**` appears on screen — so this is the way one
/// gets into the file at all.
#[test]
fn bold_wraps_the_selection_in_markers() {
    let mut p = doc("make this bold\n");
    p.anchor = Some(5);
    p.caret_at = Some(9);
    p.wrap_selection("**", 60, 20);
    assert_eq!(text_of(&p), "make **this** bold\n");
}

/// …and pressing it again takes them off, or the key would only ever
/// accumulate asterisks.
#[test]
fn bold_on_something_already_bold_takes_the_markers_off() {
    let mut p = doc("make **this** bold\n");
    p.anchor = Some(7);
    p.caret_at = Some(11);
    assert_eq!(p.selected_text().as_deref(), Some("this"));
    p.wrap_selection("**", 60, 20);
    assert_eq!(text_of(&p), "make this bold\n");
}

/// The selection has to survive the round trip, or a second press of the key
/// would wrap the wrong bytes.
#[test]
fn the_selection_still_covers_the_same_words_after_wrapping() {
    let mut p = doc("make this bold\n");
    p.anchor = Some(5);
    p.caret_at = Some(9);
    p.wrap_selection("**", 60, 20);
    assert_eq!(p.selected_text().as_deref(), Some("this"));
    p.wrap_selection("**", 60, 20);
    assert_eq!(text_of(&p), "make this bold\n", "and off again");
    assert_eq!(p.selected_text().as_deref(), Some("this"));
}

#[test]
fn italic_is_the_same_key_with_one_marker() {
    let mut p = doc("make this italic\n");
    p.anchor = Some(5);
    p.caret_at = Some(9);
    p.wrap_selection("*", 60, 20);
    assert_eq!(text_of(&p), "make *this* italic\n");
}

/// Wrapping is one undo step even though it writes two markers.
#[test]
fn wrapping_can_be_taken_back() {
    let mut p = doc("make this bold\n");
    p.anchor = Some(5);
    p.caret_at = Some(9);
    p.wrap_selection("**", 60, 20);
    while p.undo(60, 20) {}
    assert_eq!(text_of(&p), "make this bold\n");
}

/// THE bug this file's shot caught: a selection running from a heading into
/// the paragraph below it was wrapped, and since emphasis is inline the `**`
/// had no partner — markdown rendered it as two asterisks, on screen, which
/// is the one thing this editor promises never to show you.
#[test]
fn a_selection_that_leaves_its_block_is_refused_rather_than_half_wrapped() {
    let src = "# A heading\n\nAnd a paragraph under it.\n";
    let mut p = doc(src);
    p.anchor = Some(4);
    p.caret_at = Some(20);
    assert!(!p.wrap_selection("**", 60, 20), "must refuse");
    assert_eq!(text_of(&p), src, "and change nothing at all");

    // A list, a quote, a table row and a fence all start blocks too.
    for text in [
        "a line\n- an item\n",
        "a line\n> quoted\n",
        "a line\n| a | b |\n",
        "a line\n```\ncode\n```\n",
        "a line\n1. numbered\n",
        "a line\n\nnext paragraph\n",
    ] {
        let mut p = doc(text);
        p.anchor = Some(2);
        p.caret_at = Some(text.len() as u32 - 1);
        assert!(!p.wrap_selection("**", 60, 20), "must refuse: {text:?}");
        assert_eq!(text_of(&p), text);
    }
}

/// A hard-wrapped paragraph is still ONE block, and bolding across its line
/// breaks is exactly what markdown allows — refusing that would make the key
/// useless in every document written at 80 columns.
#[test]
fn a_selection_across_a_wrapped_line_of_one_paragraph_is_allowed() {
    let src = "a paragraph written\nacross two source lines\n";
    let mut p = doc(src);
    p.anchor = Some(2);
    p.caret_at = Some(25);
    assert!(p.wrap_selection("**", 60, 20));
    assert!(text_of(&p).contains("a **paragraph written\nacros**"));
}

/// The second thing the shot caught: markdown will not read `**word **` as
/// bold — a closing delimiter preceded by a space does not flank — so the
/// asterisks came out on screen. The selection is trimmed to what it is
/// actually emphasizing, which is what every other editor does too.
#[test]
fn a_selection_with_spaces_around_it_is_trimmed_before_wrapping() {
    let mut p = doc("make this word bold\n");
    p.anchor = Some(4);
    p.caret_at = Some(15);
    assert_eq!(p.selected_text().as_deref(), Some(" this word "));
    assert!(p.wrap_selection("**", 60, 20));
    assert_eq!(text_of(&p), "make **this word** bold\n");
    // …and the render agrees it is emphasis rather than four asterisks.
    let drawn: String = p
        .lines_for(60)
        .lines
        .iter()
        .flat_map(|l| l.iter().map(|c| c.c))
        .collect();
    assert!(!drawn.contains('*'), "markers on screen: {drawn:?}");
}

/// Selecting nothing but whitespace has nothing to emphasize.
#[test]
fn a_selection_of_only_spaces_is_refused() {
    let mut p = doc("a   b\n");
    p.anchor = Some(1);
    p.caret_at = Some(4);
    assert!(!p.wrap_selection("**", 60, 20));
    assert_eq!(text_of(&p), "a   b\n");
}

#[test]
fn typing_replaces_what_is_selected() {
    let mut p = doc("replace this word\n");
    p.anchor = Some(8);
    p.caret_at = Some(12);
    p.insert("that", 60, 20);
    assert_eq!(text_of(&p), "replace that word\n");
    assert_eq!(range(&p), None, "and the selection is gone");
}

#[test]
fn backspace_deletes_what_is_selected() {
    let mut p = doc("delete this word\n");
    p.anchor = Some(7);
    p.caret_at = Some(12);
    p.backspace(60, 20);
    assert_eq!(text_of(&p), "delete word\n");
}

#[test]
fn select_all_takes_the_whole_document() {
    let mut p = doc("one\n\ntwo\n");
    p.select_all(60, 20);
    assert_eq!(p.selected_text().as_deref(), Some("one\n\ntwo\n"));
}

/// A click means "put it here", which includes putting down whatever was
/// selected before.
#[test]
fn a_click_clears_the_selection() {
    let mut p = doc("hello world\n");
    select(&mut p, 5);
    assert!(range(&p).is_some());
    p.click_caret(0, 2, 60, 20);
    assert_eq!(range(&p), None);
}
