use super::highlight;
use crew_render::CellView;

fn row(text: &str, r: u16) -> Vec<CellView> {
    text.chars()
        .enumerate()
        .map(|(i, c)| CellView {
            col: i as u16,
            row: r,
            c,
            fg: (200, 200, 200),
            bg: (0, 0, 0),
            bold: false,
            italic: false,
            ..Default::default()
        })
        .collect()
}

#[test]
fn highlights_each_match_and_counts() {
    let _g = crate::app::theme_test_guard();
    // "foo bar foo" → two "foo" matches on row 0.
    let mut cells = row("foo bar foo", 0);
    let n = highlight(&mut cells, "foo", 11, 1);
    assert_eq!(n, 2);
    // exactly the 6 cells of the two matches are washed.
    let washed = cells
        .iter()
        .filter(|c| c.bg == crew_theme::theme().find_hl_bg)
        .count();
    assert_eq!(washed, 6);
    // a space between is not highlighted.
    assert!(cells
        .iter()
        .any(|c| c.c == ' ' && c.bg != crew_theme::theme().find_hl_bg));
}

#[test]
fn smart_case_matches() {
    // lowercase term → case-insensitive.
    let mut cells = row("Error: boom", 0);
    assert_eq!(highlight(&mut cells, "error", 11, 1), 1);
    // a term with uppercase → case-sensitive (no match here).
    let mut cells = row("error: boom", 0);
    assert_eq!(highlight(&mut cells, "Error", 11, 1), 0);
}

/// `/find` could not find any text holding a full-width character: the
/// grid is column-indexed, so `全角` sits on it as `全 _ 角 _` and a
/// needle written `全角` never matched.
#[test]
fn a_needle_with_a_full_width_character_matches() {
    let _g = crate::app::theme_test_guard();
    // The grid as the terminal hands it over: one cell per character,
    // the column after a wide one left blank.
    let cells_of = |pairs: &[(u16, char)]| -> Vec<CellView> {
        pairs
            .iter()
            .map(|&(col, c)| CellView {
                col,
                row: 0,
                c,
                fg: (200, 200, 200),
                bg: (0, 0, 0),
                ..Default::default()
            })
            .collect()
    };
    // `ab全角cd` — 全 at 2-3, 角 at 4-5.
    let grid = [
        (0, 'a'),
        (1, 'b'),
        (2, '\u{5168}'),
        (4, '\u{89d2}'),
        (6, 'c'),
        (7, 'd'),
    ];
    let mut cells = cells_of(&grid);
    assert_eq!(highlight(&mut cells, "\u{5168}\u{89d2}", 8, 1), 1);
    let hl = crew_theme::theme().find_hl_bg;
    let washed: Vec<u16> = cells.iter().filter(|c| c.bg == hl).map(|c| c.col).collect();
    assert_eq!(washed, vec![2, 4], "both wide cells, and only those");

    // The neighbours are not swept in with them.
    let mut cells = cells_of(&grid);
    assert_eq!(highlight(&mut cells, "b\u{5168}", 8, 1), 1);
    let washed: Vec<u16> = cells.iter().filter(|c| c.bg == hl).map(|c| c.col).collect();
    assert_eq!(washed, vec![1, 2]);
}

/// The wash replaces the background the terminal floored the ink
/// against. A match inside a TUI's painted row came out as a solid block
/// with the text invisible in it — the one thing you searched for.
#[test]
fn a_match_stays_readable_over_the_wash() {
    let _g = crate::app::theme_test_guard();
    let hl = crew_theme::theme().find_hl_bg;
    // Ink the terminal picked to read on a LIGHT painted row, matched
    // inside it: over the wash it would be invisible.
    let mut cells = row("boom", 0);
    for c in cells.iter_mut() {
        c.fg = hl;
        c.bg = (230, 240, 255);
    }
    assert_eq!(highlight(&mut cells, "boom", 4, 1), 1);
    for c in &cells {
        let r = crew_theme::contrast_ratio(c.fg, c.bg);
        assert!(
            r >= crew_theme::contrast::text_floor() - 0.05,
            "{:?} reads at {r}",
            c.fg
        );
    }
}

#[test]
fn empty_term_does_nothing() {
    let _g = crate::app::theme_test_guard();
    let mut cells = row("hello", 0);
    assert_eq!(highlight(&mut cells, "", 5, 1), 0);
    assert!(cells.iter().all(|c| c.bg != crew_theme::theme().find_hl_bg));
}
