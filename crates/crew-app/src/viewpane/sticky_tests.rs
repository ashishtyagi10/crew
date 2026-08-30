//! What the band has to say, and when it must say nothing.
use super::*;

fn marks(rows: &[(usize, u8, &str)]) -> Vec<Mark> {
    rows.iter()
        .map(|&(row, depth, label)| Mark {
            row,
            depth,
            label: label.into(),
        })
        .collect()
}

fn doc() -> Vec<Mark> {
    marks(&[
        (0, 1, "crew"),
        (10, 2, "Themes"),
        (30, 3, "CRT"),
        (60, 2, "Panes"),
    ])
}

/// The top of a file needs no address: its own first line is one.
#[test]
fn nothing_sticks_at_the_top() {
    assert_eq!(label_for(&doc(), 0), None);
}

/// A heading already on the top row must not be drawn one row above itself.
#[test]
fn nothing_sticks_when_the_heading_is_already_the_top_row() {
    assert_eq!(label_for(&doc(), 10), None);
    assert_eq!(label_for(&doc(), 30), None);
}

/// The whole point: scrolled into a section, the band says which one.
#[test]
fn the_band_names_the_section_you_are_inside() {
    assert_eq!(
        label_for(&doc(), 12).as_deref(),
        Some("crew \u{203a} Themes")
    );
}

/// …and the ladder above it, so the address is complete rather than nearest.
#[test]
fn an_inner_heading_is_shown_under_its_parents() {
    assert_eq!(
        label_for(&doc(), 40).as_deref(),
        Some("crew \u{203a} Themes \u{203a} CRT")
    );
    // Back out to a sibling of `Themes`: `CRT` is no longer above us.
    assert_eq!(
        label_for(&doc(), 70).as_deref(),
        Some("crew \u{203a} Panes")
    );
}

/// A diff's landmarks have no nesting — a hunk is after a file header, not
/// inside it — so the trail is the one landmark and nothing else.
#[test]
fn a_landmark_with_no_depth_has_no_ladder() {
    let m = marks(&[(0, 0, "src/main.rs"), (20, 0, "@@ -1,4 +1,9 @@")]);
    assert_eq!(label_for(&m, 25).as_deref(), Some("@@ -1,4 +1,9 @@"));
}

#[test]
fn a_document_with_no_headings_gets_no_band() {
    assert_eq!(label_for(&[], 40), None);
}

/// The band replaces the top row rather than adding to it: a document that
/// grew a row when you scrolled would be a document that lies about its rows.
#[test]
fn the_band_covers_the_top_row_across_the_whole_width() {
    let mut cells: Vec<CellView> = (0..12u16)
        .flat_map(|col| {
            (0..3u16).map(move |row| CellView {
                col,
                row,
                c: 'x',
                ..Default::default()
            })
        })
        .collect();
    draw(&mut cells, "crew \u{203a} Themes", 12);
    let top: Vec<&CellView> = cells.iter().filter(|c| c.row == 0).collect();
    assert_eq!(top.len(), 12, "one cell per column, no more and no less");
    assert!(top.iter().all(|c| c.c != 'x'), "the row it covered is gone");
    let text: String = {
        let mut row: Vec<&&CellView> = top.iter().collect();
        row.sort_by_key(|c| c.col);
        row.iter().map(|c| c.c).collect()
    };
    assert!(text.starts_with(" crew"), "said {text:?}");
    assert_eq!(
        cells.iter().filter(|c| c.row == 1).count(),
        12,
        "the rows below are untouched"
    );
}
