use super::*;

/// Each rendered line as text.
fn texts(src: &str, cols: usize) -> Vec<String> {
    crate::md::render(src, cols)
        .iter()
        .map(|l| l.spans.iter().map(|s| s.text.as_str()).collect())
        .collect()
}

/// The display columns the `│` separators of `row` stand on.
fn seps(row: &str) -> Vec<usize> {
    let mut at = 0;
    let mut out = Vec::new();
    for c in row.chars() {
        if c == '\u{2502}' {
            out.push(at);
        }
        at += crate::chatwidth::char_w(c);
    }
    out
}

const WIDE: &str = "| name | size | notes |\n|---|---:|---|\n\
| crew | 12 | a terminal whose markdown tables used to be cut off at the card edge |\n\
| ghostty | 9 | metal |\n";

/// On main this table at 40 columns was three lines — header, rule, the
/// one body row cut at column 40 — and `edge` appeared nowhere. Now the
/// notes column is squeezed and its text continues on rows under the
/// first, every one of them ruled by the same separators.
#[test]
fn a_wide_table_wraps_its_long_cell_onto_aligned_continuation_rows() {
    let rows = texts(WIDE, 40);
    let header = &rows[0];
    let body: Vec<&String> = rows.iter().skip(2).collect();
    assert!(
        body.len() > 2,
        "the long cell continues onto rows of its own: {rows:#?}"
    );
    for r in &rows {
        assert!(crate::chatwidth::str_w(r) <= 40, "{r:?} is wider than 40");
    }
    for r in &body {
        assert_eq!(seps(r), seps(header), "{r:?} vs header {header:?}");
    }
    let notes: String = body
        .iter()
        .take_while(|r| !r.contains("ghostty"))
        .map(|r| r.rsplit('\u{2502}').next().unwrap_or("").trim())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        notes,
        "a terminal whose markdown tables used to be cut off at the card edge"
    );
    // Continuation rows leave the short columns blank rather than
    // repeating `crew` and `12` under themselves.
    assert!(body[1].starts_with("     "), "{:?}", body[1]);
}

/// A table that fits is laid out exactly as it always was.
#[test]
fn a_table_that_fits_is_unchanged() {
    let rows = texts(WIDE, 120);
    assert_eq!(rows.len(), 4, "{rows:#?}");
    assert!(rows[2].contains("card edge"), "{:?}", rows[2]);
    assert!(rows[2].starts_with("crew    \u{2502}"), "{:?}", rows[2]);
}

/// Eight columns at twenty: even at the six-column floor they overflow, so
/// the clip stays — rows within budget, none continued.
#[test]
fn a_table_too_wide_even_at_the_floor_keeps_the_clip() {
    let src = "| a | b | c | d | e | f | g | h |\n|---|---|---|---|---|---|---|---|\n\
| one two three | 2 | 3 | 4 | 5 | 6 | 7 | 8 |\n";
    let rows = texts(src, 20);
    assert_eq!(rows.len(), 3, "{rows:#?}");
    for r in &rows {
        assert!(crate::chatwidth::str_w(r) <= 20, "{r:?}");
    }
}

/// Only the columns past the level are squeezed, and to the largest level
/// that fits: 4 + 4 + L + two separators of 3 = 40.
#[test]
fn shrink_caps_only_the_widest_columns() {
    assert_eq!(shrink(&[4, 4, 68], 6, 40), Some(vec![4, 4, 26]));
    assert_eq!(shrink(&[30, 30], 3, 40), Some(vec![18, 18]));
    assert_eq!(shrink(&[30, 30, 30, 30], 9, 30), None, "4 × 6 + 9 > 30");
}

/// A CJK cell wraps where its COLUMN ends, not at twice the width a char
/// count would allow — the separators stay put.
#[test]
fn a_wide_glyph_cell_wraps_at_its_column() {
    let src = "| a | b |\n|---|---|\n| x | 漢字漢字漢字漢字漢字漢字漢字漢字漢字漢字 |\n";
    let rows = texts(src, 16);
    assert!(rows.len() > 3, "{rows:#?}");
    for r in &rows {
        assert!(crate::chatwidth::str_w(r) <= 16, "{r:?}");
    }
    // Every row but the rule shares the header's separator.
    for r in rows.iter().skip(2) {
        assert_eq!(seps(r), seps(&rows[0]), "{r:?}");
    }
}
