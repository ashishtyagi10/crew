//! Typing into a rendered document, and the file that comes out the other
//! side. The assertion that matters most is the last one: what a save writes
//! is what was read, with exactly the edit spliced in.
use super::super::caret::Step;
use super::*;
use crate::viewpane::detect::Format;
use crate::viewpane::load::Loaded;

const DOC: &str = "\
# Title

The first paragraph, with enough words in it to wrap at a narrow width.

- one
- two

Last line.
";

fn doc(text: &str) -> ViewPane {
    let mut p = ViewPane::open(std::env::temp_dir().join("edit.md"));
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
    match &p.state {
        LoadState::Ready { loaded, .. } => loaded.text.clone(),
        _ => String::new(),
    }
}

/// Right `n` times from the start.
fn at(p: &mut ViewPane, n: usize) {
    for _ in 0..n {
        p.move_caret(Step::Right, 60, 20);
    }
}

#[test]
fn a_typed_character_lands_at_the_caret_and_nowhere_else() {
    let mut p = doc(DOC);
    p.insert("X", 60, 20);
    assert_eq!(text_of(&p), DOC.replacen("# Title", "# XTitle", 1));
    assert!(p.dirty, "the document has unsaved changes");
}

/// The caret ends up after what was typed, so a word can be typed one letter
/// at a time — the thing an editor is for.
#[test]
fn typing_a_word_puts_it_in_the_order_it_was_typed() {
    let mut p = doc(DOC);
    for c in "New ".chars() {
        p.insert(&c.to_string(), 60, 20);
    }
    assert!(
        text_of(&p).starts_with("# New Title"),
        "got {:?}",
        &text_of(&p)[..20]
    );
}

/// Typing in the middle of a document must not disturb a byte on either side
/// of it — this is the guarantee the whole design exists for.
#[test]
fn everything_but_the_edit_is_byte_for_byte_unchanged() {
    let mut p = doc(DOC);
    at(&mut p, 30);
    p.insert("!", 60, 20);
    let after = text_of(&p);
    assert_eq!(after.len(), DOC.len() + 1);
    let cut = after.find('!').expect("the typed character");
    assert_eq!(&after[..cut], &DOC[..cut], "the bytes before it moved");
    assert_eq!(&after[cut + 1..], &DOC[cut..], "the bytes after it moved");
}

#[test]
fn backspace_removes_the_character_before_the_caret() {
    let mut p = doc(DOC);
    at(&mut p, 5);
    let before = text_of(&p);
    p.backspace(60, 20);
    let after = text_of(&p);
    assert_eq!(after.len(), before.len() - 1);
    // …and typing it back gives the file that was read.
    p.insert("e", 60, 20);
    assert_eq!(
        text_of(&p),
        DOC,
        "backspace then retype was not a round trip"
    );
}

/// At the very start there is nothing to delete, and nothing may happen.
#[test]
fn backspace_at_the_start_of_the_document_does_nothing() {
    let mut p = doc("abc\n");
    let before = text_of(&p);
    p.backspace(60, 20);
    p.backspace(60, 20);
    p.backspace(60, 20);
    assert_eq!(text_of(&p), before);
}

/// A caret can stand after the last character, so a document can be added to.
#[test]
fn a_document_can_be_typed_onto_the_end_of() {
    let mut p = doc("one\n");
    p.move_caret(Step::End, 60, 20);
    p.insert("!", 60, 20);
    assert_eq!(text_of(&p), "one!\n");
}

/// Enter in prose has to leave a BLANK line: a single newline is a soft break
/// that CommonMark joins with a space, so pressing Enter would look like
/// nothing happened.
#[test]
fn enter_in_a_paragraph_starts_a_new_paragraph() {
    let mut p = doc("one two\n");
    p.move_caret(Step::End, 60, 20);
    p.newline(60, 20);
    p.insert("three", 60, 20);
    assert_eq!(text_of(&p), "one two\n\nthree\n");
    // …and the render agrees they are two paragraphs, not one line.
    let rows = p.lines_for(60).lines.len();
    assert!(rows >= 3, "two paragraphs and a blank between them: {rows}");
}

/// Enter in a list continues the list, because the alternative is a paragraph
/// interrupting it — which is what a bare newline would produce.
#[test]
fn enter_in_a_list_starts_another_item() {
    let mut p = doc("- one\n");
    p.move_caret(Step::End, 60, 20);
    p.newline(60, 20);
    p.insert("two", 60, 20);
    assert_eq!(text_of(&p), "- one\n- two\n");
}

#[test]
fn enter_in_a_numbered_list_counts_up() {
    let mut p = doc("1. one\n");
    p.move_caret(Step::End, 60, 20);
    p.newline(60, 20);
    p.insert("two", 60, 20);
    assert_eq!(text_of(&p), "1. one\n2. two\n");
}

#[test]
fn enter_keeps_a_nested_items_indent_and_a_quotes_bar() {
    let mut p = doc("- one\n  - deep\n");
    // Down to the NESTED item, then to the end of it — `at(3)` would still be
    // on the first item, where the continuation is `- ` with no indent.
    p.move_caret(Step::Down, 60, 20);
    p.move_caret(Step::End, 60, 20);
    p.newline(60, 20);
    p.insert("also", 60, 20);
    assert_eq!(text_of(&p), "- one\n  - deep\n  - also\n");

    let mut q = doc("> quoted\n");
    q.move_caret(Step::End, 60, 20);
    q.newline(60, 20);
    q.insert("more", 60, 20);
    assert_eq!(text_of(&q), "> quoted\n> more\n");
}

/// THE test. A real document, one word changed, and a diff of exactly one
/// line — no rewrapped paragraphs, no `*` bullets turned into `-`, no setext
/// heading rewritten. This is what a serializer cannot promise and a splice
/// cannot fail to do.
#[test]
fn a_save_writes_the_file_that_was_read_with_only_the_edit_in_it() {
    let original = "\
Title
=====

* a bullet written with a star
* another, in a paragraph that the renderer would happily re-wrap for you

Some prose  with  odd   spacing that no formatter should be allowed to touch.

    an indented code block

| a | b |
|--:|:--|
| 1 | 2 |
";
    let path = std::env::temp_dir().join("crew-edit-roundtrip.md");
    std::fs::write(&path, original).expect("write");
    let mut p = ViewPane::open(path.clone());
    p.state = LoadState::Ready {
        format: Format::Markdown,
        loaded: Loaded {
            text: original.into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    p.start_editing(48);
    at(&mut p, 60);
    p.insert("Z", 48, 20);
    p.save().expect("save");
    let written = std::fs::read_to_string(&path).expect("read back");
    assert_eq!(written.len(), original.len() + 1);
    let differing: Vec<(usize, &str, &str)> = written
        .lines()
        .zip(original.lines())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| (i, a, b))
        .collect();
    assert_eq!(differing.len(), 1, "changed lines: {differing:?}");
    assert!(!p.dirty, "saving clears the unsaved mark");
}

/// A save with nothing typed writes the file unchanged — an editor that
/// rewrites a file just for having been opened is not one anybody can use in
/// a repository.
#[test]
fn saving_an_untouched_document_changes_nothing() {
    let original = "Title\n=====\n\n*  odd   spacing  *\n";
    let path = std::env::temp_dir().join("crew-edit-untouched.md");
    std::fs::write(&path, original).expect("write");
    let mut p = doc(original);
    p.path = path.clone();
    p.save().expect("save");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), original);
}

/// Undo through the pane, not just the history: the file must come back
/// byte-for-byte, which is the thing a re-serializing editor cannot promise.
#[test]
fn undo_gives_the_document_back_exactly() {
    let mut p = doc(DOC);
    at(&mut p, 20);
    for c in "hello".chars() {
        p.insert(&c.to_string(), 60, 20);
    }
    assert_ne!(text_of(&p), DOC);
    assert!(p.undo(60, 20), "there was something to undo");
    assert_eq!(text_of(&p), DOC, "the file came back changed");
    assert!(!p.undo(60, 20), "and there is nothing further back");
}

#[test]
fn redo_puts_it_back() {
    let mut p = doc(DOC);
    at(&mut p, 20);
    p.insert("Q", 60, 20);
    let typed = text_of(&p);
    p.undo(60, 20);
    assert!(p.redo(60, 20));
    assert_eq!(text_of(&p), typed);
}

/// The caret comes back with the text — undoing into a document while the
/// cursor stays where the text used to be is how an editor loses your place.
#[test]
fn undo_puts_the_caret_back_where_the_typing_started() {
    let mut p = doc(DOC);
    at(&mut p, 20);
    let before = p.caret_at;
    for c in "words".chars() {
        p.insert(&c.to_string(), 60, 20);
    }
    p.undo(60, 20);
    assert_eq!(p.caret_at, before);
}

/// Backspacing is undone as a run too, and gives back what was deleted.
#[test]
fn undo_puts_back_a_run_of_backspaces() {
    let mut p = doc(DOC);
    at(&mut p, 6);
    for _ in 0..4 {
        p.backspace(60, 20);
    }
    assert_ne!(text_of(&p), DOC);
    // However the deletions were grouped into changes, undoing them all has
    // to give the file back exactly — that is the property, not the grouping.
    while p.undo(60, 20) {}
    assert_eq!(text_of(&p), DOC);
}

#[test]
fn forward_delete_takes_the_character_at_the_caret() {
    let mut p = doc("abc\n");
    p.delete(60, 20);
    assert_eq!(text_of(&p), "bc\n");
    p.undo(60, 20);
    assert_eq!(text_of(&p), "abc\n");
}

/// A click puts the caret where it landed — and on the nearest place to
/// stand when the click was past the end of a line.
#[test]
fn a_click_places_the_caret_where_it_landed() {
    let mut p = doc(DOC);
    let cols = 60;
    p.click_caret(0, 4, cols, 20);
    let on_heading = p.caret_at.expect("placed");
    assert_eq!(&DOC[on_heading as usize..on_heading as usize + 1], "l");
    // Past the end of the row: the end of it, not nothing.
    p.click_caret(0, 200, cols, 20);
    let at_end = p.caret_at.expect("placed");
    assert!(at_end > on_heading);
    assert_eq!(&DOC[..at_end as usize], "# Title");
}

/// Moving the caret by hand ends the run: what you type next is a separate
/// thing you did, and undo gives it back on its own.
#[test]
fn typing_after_a_click_is_its_own_undo_step() {
    let mut p = doc(DOC);
    p.insert("A", 60, 20);
    p.click_caret(2, 3, 60, 20);
    p.insert("B", 60, 20);
    p.undo(60, 20);
    assert!(
        text_of(&p).contains('A'),
        "the first edit survived the undo"
    );
    assert!(!text_of(&p).contains('B'));
}

/// The property the whole history exists for, over a long mixed session:
/// however the changes were grouped, undoing all of them gives back the file
/// that was read, byte for byte.
#[test]
fn undoing_everything_gives_back_the_file_that_was_read() {
    let mut p = doc(DOC);
    at(&mut p, 12);
    for c in "one two\nthree ".chars() {
        match c {
            '\n' => p.newline(60, 20),
            _ => p.insert(&c.to_string(), 60, 20),
        }
    }
    for _ in 0..5 {
        p.backspace(60, 20);
    }
    p.move_caret(Step::Down, 60, 20);
    p.insert("more", 60, 20);
    p.delete(60, 20);
    assert_ne!(text_of(&p), DOC, "the session actually changed something");
    while p.undo(60, 20) {}
    assert_eq!(text_of(&p), DOC);
    // …and redoing all of it returns to where the session ended.
    let undone = text_of(&p);
    while p.redo(60, 20) {}
    assert_ne!(text_of(&p), undone, "redo put the session back");
    while p.undo(60, 20) {}
    assert_eq!(text_of(&p), DOC, "and undo still gives the file back");
}
