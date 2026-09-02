//! The columns a list shares between its rows — found by shooting the attach
//! picker and the two colour pickers (`pickshot_tests`).
use super::{label_col, spans, swatch_col};
use crate::suggest::MenuItem;
use ratatui::style::Color;
use ratatui::text::Line;

const DIM: Color = Color::Rgb(120, 130, 140);

fn item(label: &str, desc: &str) -> MenuItem {
    MenuItem {
        label: label.into(),
        desc: desc.into(),
        ..Default::default()
    }
}

fn text(l: &Line<'static>) -> String {
    l.spans.iter().map(|s| s.content.as_ref()).collect()
}

/// The column `needle` starts at — in cells, not bytes (a chip is 3 bytes).
fn col_of(l: &Line<'static>, needle: &str) -> Option<usize> {
    let t = text(l);
    t.find(needle).map(|b| t[..b].chars().count())
}

fn chip() -> crate::swatch::Chip {
    crate::swatch::Chip {
        c: '\u{2588}',
        fg: (1, 2, 3),
        bg: None,
    }
}

/// `@a+b  fans the task out to both, in parallel` ended `in para` on a
/// quarter-width tile, cut by the card with nothing to say so.
#[test]
fn a_header_wider_than_the_card_marks_its_cut() {
    let mut h = item("@a+b  fans the task out to both, in parallel", "");
    h.header = true;
    let line = text(&spans(&h, 8, 0, 20, DIM));
    assert!(line.ends_with('\u{2026}'), "{line:?}");
    assert_eq!(line.chars().count(), 20);
}

/// The attach picker lists agents (with a role) above files (with nothing):
/// the longest path was setting where every role started.
#[test]
fn rows_without_a_description_do_not_set_the_column() {
    let rows = [
        item("@coder", "agent \u{b7} writes code"),
        item("@planner", "agent \u{b7} plans"),
        item("@README.md", ""),
        item("@crates/crew-app/src/viewpane/render_tests.rs", ""),
    ];
    assert_eq!(label_col(&rows, 80), 8, "the widest DESCRIBED label");
    let w = label_col(&rows, 80);
    let at = |r: &MenuItem| col_of(&spans(r, w, 0, 80, DIM), "agent");
    assert_eq!(at(&rows[0]), Some(10));
    assert_eq!(at(&rows[1]), Some(10));
}

/// `/gradient`'s `subtle` has no colour and `aurora` has four cells of it;
/// `/theme`'s modes show four chips and its palettes six. Every description
/// in one list starts in one column regardless.
#[test]
fn rows_with_fewer_or_no_chips_keep_the_description_column() {
    let mut four = item("aurora", "teal into violet");
    four.swatch = vec![chip(); 4];
    let mut one = item("mono", "no colour");
    one.swatch = vec![chip()];
    let none = item("subtle", "the default");
    let rows = [four, one, none];
    assert_eq!(swatch_col(&rows), 4);
    let (lw, sw) = (label_col(&rows, 60), swatch_col(&rows));
    let start = |r: &MenuItem, needle: &str| col_of(&spans(r, lw, sw, 60, DIM), needle);
    let col = start(&rows[0], "teal").unwrap();
    assert_eq!(start(&rows[1], "no colour"), Some(col), "one chip");
    assert_eq!(start(&rows[2], "the default"), Some(col), "no chip");
    // The row's own swatch still draws when the list-wide width is not known.
    assert_eq!(
        text(&spans(&rows[0], lw, 0, 60, DIM))
            .matches('\u{2588}')
            .count(),
        4
    );
}
