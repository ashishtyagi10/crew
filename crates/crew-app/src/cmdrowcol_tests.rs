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

/// A description too long for its row is cut between words, as the header
/// row is: "bump, tag and p…" read as a mangled word. Every width from a
/// narrow card to a wide one, so no width can land on a space by luck.
#[test]
fn a_cut_description_ends_on_a_word() {
    let desc = "bump the version, tag it and push the release";
    for avail in 24..48 {
        let line = text(&spans(&item("/release", desc), 10, 0, avail, DIM));
        let Some(body) = line.trim_end().strip_suffix('\u{2026}') else {
            continue;
        };
        assert!(
            desc.split([' ', ','])
                .any(|w| !w.is_empty() && body.ends_with(w)),
            "{avail}: cut mid-word: {line:?}"
        );
    }
}

/// The `@a+b` note was one string with two spaces in it: its sentence began
/// four columns after the selector, while the roles of the agents right
/// above it began at the description column. It starts there too now — and
/// on a card too narrow for both columns it is still one line cut on a word.
#[test]
fn the_fan_out_note_starts_in_the_description_column() {
    use crate::chatmention::MentionEntry;
    let agent = |name: &str, role: &str| MentionEntry::Agent {
        name: name.into(),
        role: role.into(),
    };
    let entries = [agent("planner", "planning"), agent("coder", "building")];
    let rows = crate::chatpalette::chatpaletteitems::attach_items("", &entries, false);
    let lw = label_col(&rows, 60);
    let col = |needle: &str| {
        let r = rows
            .iter()
            .find(|r| text(&spans(r, lw, 0, 60, DIM)).contains(needle));
        col_of(&spans(r.unwrap(), lw, 0, 60, DIM), needle)
    };
    assert_eq!(col("fans the task"), col("agent \u{b7} planning"));
    assert!(super::content_w(&rows) >= 2 + col("in parallel").unwrap() + 11);
    let note = rows.iter().find(|r| r.header).unwrap();
    let narrow = spans(note, lw, 0, 24, DIM);
    assert_eq!(col_of(&narrow, "fans"), col("agent \u{b7} planning"));
    assert_eq!(text(&narrow), "@a+b      fans the task\u{2026}");
    assert_eq!(text(&spans(note, lw, 0, 16, DIM)), "@a+b  fans the\u{2026}");
}
