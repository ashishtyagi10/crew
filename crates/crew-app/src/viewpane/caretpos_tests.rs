//! The line and column are the file's, not the render's.
use crate::viewpane::caret::Step;
use crate::viewpane::detect::Format;
use crate::viewpane::editpane_tests::{doc, DOC};
use crate::viewpane::load::Loaded;
use crate::viewpane::{LoadState, ViewPane};

#[test]
fn a_viewer_has_no_position_and_an_editor_starts_after_the_heading_marker() {
    let mut p = doc();
    assert_eq!(p.caret_line_col(), None, "no caret, no position");
    p.start_editing(60);
    // The caret is on the `T` of `# The` — byte 2, so column 3: the `# ` is
    // in the file even though it is not on the screen.
    assert_eq!(p.caret_line_col(), Some((1, 3)));
    for _ in 0..3 {
        p.move_caret(Step::Right, 60, 20);
    }
    assert_eq!(p.caret_line_col(), Some((1, 6)));
}

#[test]
fn the_documents_end_is_its_last_line() {
    let mut p = doc();
    p.start_editing(60);
    p.move_caret(Step::Bottom, 60, 20);
    let last = DOC.lines().last().expect("a last line");
    assert_eq!(
        p.caret_line_col(),
        Some((DOC.lines().count(), last.chars().count() + 1)),
        "after the last character of the last line"
    );
}

/// Bytes would say 5; a person counting characters says 4.
#[test]
fn the_column_counts_characters_not_bytes() {
    let mut p = ViewPane::open(std::env::temp_dir().join("accent.md"));
    p.state = LoadState::Ready {
        format: Format::Markdown,
        loaded: Loaded {
            text: "# é\n\nx\n".into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    p.start_editing(60);
    assert_eq!(p.caret_line_col(), Some((1, 3)), "on the é");
    p.move_caret(Step::End, 60, 20);
    assert_eq!(p.caret_line_col(), Some((1, 4)), "after it");
    p.move_caret(Step::Bottom, 60, 20);
    assert_eq!(p.caret_line_col(), Some((3, 2)));
}
