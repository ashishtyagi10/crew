//! Shooting the input bar: render it and read the cells back.
//!
//! The most-used surface in the app had no test of any kind. These render it
//! at real widths and assert what a person would check by looking — that
//! nothing overlaps, nothing escapes the card, and the things that are
//! supposed to be there are.
use super::*;

/// The rendered grid as rows of text, `cols` wide, spaces where nothing was
/// drawn. Cells are emitted in no particular order and wide glyphs claim two
/// columns, so this is the only honest way to ask what the frame says.
fn rows_of(cells: &[CellView], cols: u16, rows: u16) -> Vec<String> {
    let mut grid = vec![vec![' '; cols as usize]; rows as usize];
    for c in cells {
        if let Some(r) = grid.get_mut(c.row as usize) {
            if let Some(slot) = r.get_mut(c.col as usize) {
                *slot = c.c;
            }
        }
    }
    grid.into_iter().map(|r| r.into_iter().collect()).collect()
}

fn bar(text: &str, focused: bool) -> InputBar {
    InputBar {
        text: text.to_string(),
        focused,
        cwd: "/Users/x/code/crew".into(),
        ..Default::default()
    }
}

/// No cell may land outside the card, at any width. A single stray column is
/// a glyph drawn over the border or over the pane next door.
#[test]
fn nothing_is_drawn_outside_the_card_at_any_width() {
    let _g = crate::app::theme_test_guard();
    for cols in [6u16, 7, 10, 20, 40, 80, 200] {
        for rows in [3u16, 4, 5] {
            let b = bar("git commit -m 'a rather long message here'", true);
            let cells = b.cells(cols, rows, None, Some("saved"), Some("shell"));
            for c in &cells {
                assert!(
                    c.col < cols && c.row < rows,
                    "cell ({},{}) outside {cols}x{rows}",
                    c.col,
                    c.row
                );
            }
        }
    }
}

/// Two glyphs in one cell on the TEXT ROW is a collision: the later draw wins
/// and what you typed is silently mangled. (The border rows are a different
/// matter — a legend rides the rule and overwrites it on purpose; see
/// `a_border_tag_wins_the_cells_it_rides`.)
#[test]
fn nothing_collides_on_the_row_you_type_into() {
    let _g = crate::app::theme_test_guard();
    for cols in [6u16, 8, 12, 24, 48, 100] {
        for text in [
            "",
            "g",
            "git commit -m x",
            "\u{65e5}\u{672c}\u{8a9e} wide",
            "/tools",
        ] {
            let b = bar(text, true);
            let cells = b.cells(cols, 3, None, Some("done"), Some("agent smith"));
            let row = cols_written(&cells, 1);
            let mut seen = std::collections::HashSet::new();
            for c in row {
                assert!(
                    seen.insert(c),
                    "two glyphs at col {c} on the text row, cols={cols}, text={text:?}"
                );
            }
        }
    }
}

/// The bottom rule's tag is drawn AFTER the card, over the border it rides —
/// which only reads correctly because later cells paint over earlier ones.
/// Pinning it here so the ordering is a stated contract rather than an
/// accident of two call sites' sequence.
#[test]
fn a_border_tag_wins_the_cells_it_rides() {
    let _g = crate::app::theme_test_guard();
    let b = bar("x", true);
    let cells = b.cells(40, 3, None, Some("saved"), None);
    let bottom = &rows_of(&cells, 40, 3)[2];
    assert!(
        bottom.contains("saved"),
        "the tag lost to the border: {bottom:?}"
    );
}

/// Every column the frame wrote on `row`, in draw order.
fn cols_written(cells: &[CellView], row: u16) -> Vec<u16> {
    cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| c.col)
        .collect()
}

#[test]
fn a_tiny_bar_renders_nothing_rather_than_a_broken_card() {
    let _g = crate::app::theme_test_guard();
    assert!(bar("x", true).cells(5, 3, None, None, None).is_empty());
    assert!(bar("x", true).cells(40, 2, None, None, None).is_empty());
}

/// A line longer than the field shows its TAIL, and the prompt gutter carries
/// the ellipsis that says the head exists.
#[test]
fn an_overlong_line_shows_its_tail_and_says_so() {
    let _g = crate::app::theme_test_guard();
    let b = bar("abcdefghijklmnopqrstuvwxyz0123456789", true);
    let text = rows_of(&b.cells(20, 3, None, None, None), 20, 3)[1].clone();
    assert!(text.contains('\u{2026}'), "no head marker: {text:?}");
    assert!(text.contains("789"), "not the tail: {text:?}");
    assert!(!text.contains("abc"), "showed the head: {text:?}");
}

#[test]
fn the_placeholder_appears_only_on_an_empty_focused_bar() {
    let _g = crate::app::theme_test_guard();
    let shown = |b: InputBar| {
        rows_of(&b.cells(60, 3, None, None, None), 60, 3)[1].contains("type / for commands")
    };
    assert!(shown(bar("", true)));
    assert!(!shown(bar("", false)), "an unfocused bar is not prompting");
    assert!(!shown(bar("g", true)), "typing replaces the hint");
}
