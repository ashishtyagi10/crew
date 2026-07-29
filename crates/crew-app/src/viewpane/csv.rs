//! The CSV rung. Row parsing is here; column widths, padding and the header
//! rule are `md/table.rs`'s, reached through `md::table_lines` — this is a
//! CSV *adapter*, not a second table renderer.
//!
//! Limitation, deliberate: a quoted field containing a newline splits across
//! rows. Handling it needs a streaming parser, and a viewer that shows one
//! odd row imperfectly is a better trade than a dependency.
use crate::chatbody::CardLine;
use crate::md::{MdSpan, MdStyle};

/// Split `text` into rows of fields, honouring `"` quoting and `""` escapes.
pub(crate) fn parse(text: &str, delim: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            continue;
        }
        let mut fields = Vec::new();
        let mut cur = String::new();
        let mut quoted = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' if quoted && chars.peek() == Some(&'"') => {
                    cur.push('"');
                    chars.next();
                }
                '"' => quoted = !quoted,
                c if c == delim && !quoted => fields.push(std::mem::take(&mut cur)),
                c => cur.push(c),
            }
        }
        fields.push(cur);
        rows.push(fields);
    }
    rows
}

fn spans(field: &str) -> Vec<MdSpan> {
    vec![MdSpan {
        text: field.to_string(),
        style: MdStyle::default(),
        link: None,
    }]
}

/// A column-aligned table for `cols` columns; the first row is the header.
pub(crate) fn lines(text: &str, delim: char, cols: usize) -> Vec<CardLine> {
    let rows = parse(text, delim);
    let Some((head, body)) = rows.split_first() else {
        return Vec::new();
    };
    let header: Vec<Vec<MdSpan>> = head.iter().map(|f| spans(f)).collect();
    let body: Vec<Vec<Vec<MdSpan>>> = body
        .iter()
        .map(|r| r.iter().map(|f| spans(f)).collect())
        .collect();
    let fg = crew_theme::theme().ink;
    let content_w = cols.saturating_sub(1);
    crate::chatmd::map_lines(
        crate::md::table_lines(header, body, content_w),
        content_w,
        fg,
    )
}

#[cfg(test)]
#[path = "csv_tests.rs"]
mod tests;
