use super::*;

fn item(label: &str, desc: &str, hit: Vec<usize>, key: Option<&'static str>) -> MenuItem {
    MenuItem {
        label: label.into(),
        desc: desc.into(),
        hit,
        key,
        ..Default::default()
    }
}

fn text(l: &Line<'static>) -> String {
    l.spans.iter().map(|s| s.content.as_ref()).collect()
}

/// Characters the row drew in bold — the marked ones.
fn bold(l: &Line<'static>) -> String {
    l.spans
        .iter()
        .filter(|s| s.style.add_modifier.contains(Modifier::BOLD))
        .map(|s| s.content.as_ref())
        .collect()
}

const DIM: Color = Color::Rgb(120, 130, 140);

#[test]
fn the_matched_characters_are_the_marked_ones() {
    let row = item("/dump", "Dump the grid", vec![1, 3, 4], None);
    assert_eq!(bold(&spans(&row, 8, 40, DIM)), "dmp");
}

/// A match list from a longer label must not mark characters off the end.
#[test]
fn a_position_past_the_label_marks_nothing() {
    let row = item("/x", "Hi", vec![0, 9], None);
    assert_eq!(bold(&spans(&row, 4, 20, DIM)), "/");
}

#[test]
fn an_unmatched_row_marks_nothing() {
    let row = item("/dump", "Dump the grid", vec![], None);
    assert_eq!(bold(&spans(&row, 8, 40, DIM)), "");
}

/// The whole point of the column: every description starts at the same
/// character, whatever the label's length.
#[test]
fn descriptions_start_at_the_same_column_on_every_row() {
    let rows = [
        item("/a", "First", vec![], None),
        item("/longer", "Second", vec![], None),
    ];
    let w = label_col(&rows, 60);
    let at = |r: &MenuItem| text(&spans(r, w, 60, DIM)).find(char::is_uppercase);
    assert_eq!(at(&rows[0]), at(&rows[1]));
    assert_eq!(at(&rows[0]), Some(w + GAP));
}

/// A label wider than the column keeps its description one gap after itself
/// rather than being cut in half by the column it overflowed.
#[test]
fn an_overlong_label_pushes_its_own_description_instead_of_being_clipped() {
    let row = item("/an-extremely-long-command", "Desc", vec![], None);
    let line = text(&spans(&row, 6, 60, DIM));
    assert!(line.starts_with("/an-extremely-long-command"));
    assert!(line.ends_with("  Desc"), "{line}");
}

/// One long command must not eat the row: the column is capped, so short
/// commands' descriptions still start early.
#[test]
fn the_label_column_is_capped_at_half_the_row() {
    let rows = [item(&"/x".repeat(40), "d", vec![], None)];
    assert_eq!(label_col(&rows, 40), 20);
}

#[test]
fn the_chord_is_flush_with_the_right_edge() {
    let row = item("/clear", "Clear the pane", vec![], Some("Cmd+K"));
    let line = text(&spans(&row, 8, 40, DIM));
    assert_eq!(line.chars().count(), 40, "{line:?}");
    assert!(line.ends_with("Cmd+K"), "{line:?}");
}

/// A row too narrow for both drops the chord, not the description: the
/// description is what the row is for.
#[test]
fn a_narrow_row_keeps_the_description_and_drops_the_chord() {
    let row = item("/clear", "Clear the pane", vec![], Some("Cmd+K"));
    let line = text(&spans(&row, 8, 18, DIM));
    assert!(!line.contains("Cmd+K"), "{line:?}");
    assert!(line.contains("Clear"), "{line:?}");
    assert!(line.chars().count() <= 18, "{line:?}");
}

/// Whatever the width, a row never draws past the card's interior.
#[test]
fn no_row_ever_exceeds_the_columns_it_was_given() {
    let rows = [
        item(
            "/clear",
            "Clear the focused pane's scrollback",
            vec![0],
            Some("Cmd+K"),
        ),
        item("/x", "", vec![], Some("Cmd+K")),
        item("/section", "", vec![], None),
    ];
    for avail in 1..=80usize {
        let w = label_col(&rows, avail);
        for r in &rows {
            let n = text(&spans(r, w, avail, DIM)).chars().count();
            assert!(n <= avail, "{:?} took {n} of {avail}", r.label);
        }
    }
}

/// A section title is still a bare bold line — no column, no chord.
#[test]
fn a_header_row_is_left_alone() {
    let mut h = item("your subscriptions", "ignored", vec![], Some("Cmd+K"));
    h.header = true;
    assert_eq!(text(&spans(&h, 8, 40, DIM)), "your subscriptions");
}

fn chip(fg: (u8, u8, u8)) -> crate::swatch::Chip {
    crate::swatch::Chip {
        c: '\u{2588}',
        fg,
        bg: None,
    }
}

/// A colour row draws its colours, between the label column and the prose.
#[test]
fn a_swatch_is_drawn_after_the_label_and_before_the_description() {
    let mut row = item("aurora", "teal into violet", vec![], None);
    row.swatch = vec![chip((1, 2, 3)), chip((4, 5, 6))];
    let line = spans(&row, 8, 40, DIM);
    let text = text(&line);
    let block = text.find('\u{2588}').expect("no swatch drawn");
    assert!(block > text.find("aurora").unwrap());
    assert!(block < text.find("teal").unwrap());
    assert_eq!(text.matches('\u{2588}').count(), 2);
    assert!(text.chars().count() <= 40);
}

/// The cells carry the colours themselves, not a colour the row happened to
/// already be using.
#[test]
fn each_swatch_cell_keeps_its_own_colour() {
    let mut row = item("aurora", "d", vec![], None);
    row.swatch = vec![chip((10, 20, 30)), chip((40, 50, 60))];
    let line = spans(&row, 8, 40, DIM);
    let blocks: Vec<&ratatui::text::Span> = line
        .spans
        .iter()
        .filter(|s| s.content.contains('\u{2588}'))
        .collect();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].style.fg, Some(Color::Rgb(10, 20, 30)));
    assert_eq!(blocks[1].style.fg, Some(Color::Rgb(40, 50, 60)));
}

/// A two-colour chip keeps its background too — that is what makes the dark
/// pool's chips distinguishable at all.
#[test]
fn a_chip_with_a_page_colour_draws_it_as_the_cell_background() {
    let mut row = item("dark", "rotating dark pages", vec![], None);
    row.swatch = vec![crate::swatch::Chip {
        c: '\u{2580}',
        fg: (200, 100, 50),
        bg: Some((9, 9, 12)),
    }];
    let line = spans(&row, 6, 40, DIM);
    let s = line
        .spans
        .iter()
        .find(|s| s.content.contains('\u{2580}'))
        .expect("no chip drawn");
    assert_eq!(s.style.fg, Some(Color::Rgb(200, 100, 50)));
    assert_eq!(s.style.bg, Some(Color::Rgb(9, 9, 12)));
}

/// Narrow cards: the swatch is dropped rather than drawn past the edge, and
/// nothing else overruns either.
#[test]
fn a_row_with_a_swatch_still_never_exceeds_its_columns() {
    let mut row = item("aurora", "teal into violet", vec![0], Some("Cmd+K"));
    row.swatch = vec![chip((1, 2, 3)); 4];
    for avail in 1..=80usize {
        let n = text(&spans(&row, 6, avail, DIM)).chars().count();
        assert!(n <= avail, "took {n} of {avail}");
    }
}
