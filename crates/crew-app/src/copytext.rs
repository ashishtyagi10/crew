//! A row of cells read back as the text it shows, for the clipboard: each
//! character with the marks it carries (`CellView::marks`), and the column a
//! full-width character's second half owns left out. Read as one char per
//! column, `क्ष` pasted as `कष` and `日本` as `日 本`.
use std::collections::BTreeMap;

use crew_theme::marks::Marks;

/// One row's characters by column.
pub(crate) type Row = BTreeMap<u16, (char, Marks)>;

/// Rows by row number, from `(row, col, char, marks)` cells.
pub(crate) fn rows(cells: impl IntoIterator<Item = (u16, u16, char, Marks)>) -> BTreeMap<u16, Row> {
    let mut out: BTreeMap<u16, Row> = BTreeMap::new();
    for (row, col, c, marks) in cells {
        out.entry(row).or_default().insert(col, (c, marks));
    }
    out
}

/// Columns `from..=to` of `row` as text: a blank column is a space, except
/// the one a wide character spills into. Trailing blanks are trimmed.
pub(crate) fn line(row: &Row, from: u16, to: u16) -> String {
    let mut out = String::new();
    let mut tail = false;
    for col in from..=to {
        match row.get(&col) {
            Some(&(c, marks)) => {
                out.push(c);
                out.extend(marks.chars());
                tail = crate::chatwidth::char_w(c) == 2;
            }
            None if std::mem::take(&mut tail) => {}
            None => out.push(' '),
        }
    }
    out.trim_end().to_string()
}

#[cfg(test)]
#[path = "copytext_tests.rs"]
mod tests;
