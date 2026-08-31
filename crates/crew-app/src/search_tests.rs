use super::*;

fn cell(col: u16, row: u16, c: char) -> RenderCell {
    RenderCell {
        col,
        row,
        c,
        fg: (0, 0, 0),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
        ..Default::default()
    }
}

#[test]
fn grid_contains_finds_substring_in_a_row() {
    // "hi" on row 1.
    let cells = [cell(0, 1, 'h'), cell(1, 1, 'i')];
    assert!(grid_contains(&cells, "hi", 10, 3));
    assert!(!grid_contains(&cells, "bye", 10, 3));
    assert!(!grid_contains(&cells, "", 10, 3));
}

#[test]
fn grid_contains_smart_case() {
    // "Hello" on row 0.
    let cells = [
        cell(0, 0, 'H'),
        cell(1, 0, 'e'),
        cell(2, 0, 'l'),
        cell(3, 0, 'l'),
        cell(4, 0, 'o'),
    ];
    // all-lowercase term → case-insensitive, matches.
    assert!(grid_contains(&cells, "hello", 10, 1));
    assert!(grid_contains(&cells, "ell", 10, 1));
    // a term with an uppercase letter → case-sensitive.
    assert!(grid_contains(&cells, "Hello", 10, 1));
    assert!(!grid_contains(&cells, "HELLO", 10, 1));
}

#[test]
fn count_in_grid_counts_all_occurrences() {
    // "a a" on row 0 (cols 0 and 2) and "a" on row 1 → three matches total.
    let cells = [cell(0, 0, 'a'), cell(2, 0, 'a'), cell(0, 1, 'a')];
    assert_eq!(count_in_grid(&cells, "a", 10, 2), 3);
    // smart-case: lowercase term counts case-insensitively.
    let caps = [cell(0, 0, 'A'), cell(1, 0, 'b')];
    assert_eq!(count_in_grid(&caps, "ab", 10, 1), 1);
    // an uppercase term is case-sensitive (no match), and empty term is zero.
    assert_eq!(count_in_grid(&caps, "AB", 10, 1), 0);
    assert_eq!(count_in_grid(&caps, "", 10, 1), 0);
}
