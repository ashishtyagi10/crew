//! The caret as the pane holds it: moving it, keeping it on screen, and
//! keeping it on the same BYTE when the document is laid out again.
use super::caret::Step;
use super::{LoadState, ViewPane};
use crate::viewpane::detect::Format;
use crate::viewpane::load::Loaded;

const DOC: &str = "\
# The document window

A document wants a window you can put on the other screen, size to a
comfortable measure, and leave open while the grid goes on being a grid.

- one file, framed, filling it
- no nav, no input bar, no tiles

The last paragraph, which is here so there is somewhere below to scroll to
and somewhere above to scroll back from.
";

fn doc() -> ViewPane {
    let mut p = ViewPane::open(std::env::temp_dir().join("caret.md"));
    p.state = LoadState::Ready {
        format: Format::Markdown,
        loaded: Loaded {
            text: DOC.into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    p
}

/// The document opens with a cursor already in it — that IS the difference
/// between a viewer and an editor.
#[test]
fn editing_starts_on_the_first_character() {
    let mut p = doc();
    assert_eq!(p.caret, None, "a viewer pane has no caret");
    p.start_editing(60);
    assert!(p.caret.is_some());
    assert_eq!(p.caret_at, Some(2), "`# ` is two bytes of source");
}

#[test]
fn the_arrows_move_it_and_the_byte_follows() {
    let mut p = doc();
    p.start_editing(60);
    let first = p.caret_at;
    p.move_caret(Step::Right, 60, 20);
    assert!(p.caret_at > first, "{:?} then {:?}", first, p.caret_at);
    p.move_caret(Step::Left, 60, 20);
    assert_eq!(p.caret_at, first, "and back");
}

/// A caret you cannot see is a caret you cannot type at.
#[test]
fn the_document_scrolls_to_keep_the_caret_in_view() {
    let mut p = doc();
    p.start_editing(60);
    assert_eq!(p.scroll, 0);
    for _ in 0..40 {
        p.move_caret(Step::Down, 60, 6);
    }
    let c = p.caret.expect("still there");
    assert!(
        c.row >= p.scroll && c.row < p.scroll + 6,
        "caret at row {} with the window showing {}..{}",
        c.row,
        p.scroll,
        p.scroll + 6
    );
    // …and back up again, which is the direction a one-sided implementation
    // gets wrong.
    for _ in 0..40 {
        p.move_caret(Step::Up, 60, 6);
    }
    let c = p.caret.expect("still there");
    assert!(c.row >= p.scroll && c.row < p.scroll + 6, "row {}", c.row);
}

/// The byte is what the caret is; the row and column are only where this
/// width happens to put it. Resizing the window must not move the cursor to a
/// different word.
#[test]
fn a_relayout_keeps_the_caret_on_the_same_byte() {
    let mut p = doc();
    p.start_editing(70);
    // Far enough in that the two widths genuinely disagree about which row
    // this byte is on — the guard below fails if they do not.
    for _ in 0..160 {
        p.move_caret(Step::Right, 70, 20);
    }
    let at = p.caret_at.expect("an offset");
    let row_before = p.caret.expect("a caret").row;
    // The window is dragged narrower: every line re-wraps.
    p.clamp_scroll(24, 20);
    p.relayout_caret(24, 20);
    assert_eq!(p.caret_at, Some(at), "the caret changed bytes on a resize");
    let row_after = p.caret.expect("a caret").row;
    assert!(
        row_after != row_before,
        "the fixture did not actually re-wrap (row {row_before} both times)"
    );
}

/// Nothing on the read-only path grew a cursor: a viewer pane's arrows still
/// scroll, and `move_caret` on a pane that is not editing does nothing at all.
#[test]
fn a_pane_that_is_not_editing_ignores_the_caret_keys() {
    let mut p = doc();
    p.move_caret(Step::Down, 60, 20);
    assert_eq!(p.caret, None);
    assert_eq!(p.scroll, 0, "and did not scroll instead");
}

/// PageDown is a page of the window, not a page of some constant: five rows
/// is exactly what five Down presses would have done, and the view follows.
#[test]
fn a_page_down_is_that_many_downs_and_stays_in_view() {
    let mut by_hand = doc();
    by_hand.start_editing(60);
    for _ in 0..5 {
        by_hand.move_caret(Step::Down, 60, 6);
    }
    let mut p = doc();
    p.start_editing(60);
    p.move_caret(
        Step::Page {
            down: true,
            rows: 5,
        },
        60,
        6,
    );
    let c = p.caret.expect("still there");
    assert_eq!(Some(c), by_hand.caret);
    assert!(
        c.row > 0 && c.row >= p.scroll && c.row < p.scroll + 6,
        "row {} from {}",
        c.row,
        p.scroll
    );
    assert!(p.scroll > 0, "the window scrolled to follow");
    p.move_caret(Step::Bottom, 60, 6);
    let end = p.caret.expect("still there");
    assert!(
        end.row > c.row && end.row >= p.scroll && end.row < p.scroll + 6,
        "row {} from {}",
        end.row,
        p.scroll
    );
}
