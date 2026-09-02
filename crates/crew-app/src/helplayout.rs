//! What `/keys` actually puts on each row, before any of it is coloured.
//!
//! Split out of [`crate::help`] because the panel had two width bugs that are
//! both layout, not rendering:
//!
//! * The key column was the constant 26, and `{left:<26}` pads to a minimum —
//!   it does not guarantee a *gap*. Five bindings are wider than 26, so their
//!   descriptions ran straight into the keys: `Cmd+wheelFont size + / - /
//!   reset`, `Triple-clickSelect the word`. The column is now measured from
//!   the widest key there is, and a key that still overruns gets two spaces
//!   of its own.
//! * Descriptions were handed to ratatui, which clips a `Line` without a word
//!   of complaint. At any window narrower than [`crate::help::size`] asks for,
//!   half the panel read "Find: in a chat transcript, or /find in the ba".
//!   That is this module's whole reason to exist: the description *wraps*,
//!   indented under itself, and nothing is lost. The list scrolls, so extra
//!   rows cost nothing (the v0.6.52 lesson, which the width learned once and
//!   then only at the preferred size).
use crate::chatlayout::wrap_indices;
use crate::chatwidth::str_w;
use crate::helppanes::{
    DISK_BINDINGS, DOC_BINDINGS, FAR_BINDINGS, SETTINGS_BINDINGS, TODO_BINDINGS, VIEW_BINDINGS,
};
use crate::helptable::{BINDINGS, CHAT_BINDINGS};

/// The key column crew prefers: wide enough for nearly every binding, narrow
/// enough that a description is still a sentence.
pub(crate) const KEY_COL: usize = 26;

/// One display line of the panel.
pub(crate) enum Row {
    /// The blank between two sections.
    Spacer,
    /// A section title (`in an agent pane`).
    Head(&'static str),
    /// A binding: keys in the left column, the first line of its description.
    Bind(&'static str, String),
    /// A wrapped continuation, drawn indented under the description.
    Cont(String),
    /// The panel talking to you — "no binding matches …".
    Note(String),
}

/// The per-pane sections, in the order they are listed. One place, so adding a
/// pane kind is one row here and nothing else — the height, the width, the
/// scrolling and the filter all read this.
pub(crate) fn sections() -> [(&'static str, &'static [(&'static str, &'static str)]); 7] {
    [
        ("in an agent pane", CHAT_BINDINGS),
        ("in the file viewer", VIEW_BINDINGS),
        ("in a /far file panel", FAR_BINDINGS),
        ("in the /todo pane", TODO_BINDINGS),
        ("in /settings", SETTINGS_BINDINGS),
        ("in a document window", DOC_BINDINGS),
        ("in the /disk map", DISK_BINDINGS),
    ]
}

/// Every logical row, in order, as `(keys, description)` — with the spacers
/// and section headings in their places.
pub(crate) fn logical() -> Vec<(&'static str, &'static str)> {
    let mut v: Vec<(&str, &str)> = BINDINGS.to_vec();
    for (title, table) in sections() {
        v.push(("", ""));
        v.push(("", title));
        v.extend_from_slice(table);
    }
    v
}

/// The widest key in any table — what an aligned column has to clear.
pub(crate) fn widest_key() -> usize {
    logical()
        .iter()
        .filter(|(k, _)| !k.is_empty())
        .map(|(k, _)| str_w(k))
        .max()
        .unwrap_or(KEY_COL)
}

/// Where descriptions start, at a panel `cols` cells wide. Wide enough for
/// every key when the panel can afford it; never more than 45% of the panel,
/// because the description is the half that teaches.
pub(crate) fn key_col(cols: u16) -> usize {
    let cap = ((cols as usize) * 45 / 100).max(6);
    (widest_key() + 2).clamp(KEY_COL.min(cap), cap)
}

/// The rows a search shows: every binding whose keys or description contain
/// `needle`, case-insensitively.
///
/// A heading survives only when something under it did — a section title over
/// no rows is a lie about where you are in the list — and the blank spacers go
/// with them, since a filtered list has nothing to separate.
pub(crate) fn filtered(needle: &str) -> Vec<(&'static str, &'static str)> {
    let all = logical();
    if needle.is_empty() {
        return all;
    }
    let n = needle.to_lowercase();
    let hit = |(k, d): &(&str, &str)| {
        !k.is_empty() && (k.to_lowercase().contains(&n) || d.to_lowercase().contains(&n))
    };
    let mut out: Vec<(&'static str, &'static str)> = Vec::new();
    for (i, row) in all.iter().enumerate() {
        if hit(row) {
            // Carry the heading this row sits under, once.
            if let Some(head) = all[..i]
                .iter()
                .rev()
                .find(|(k, d)| k.is_empty() && !d.is_empty())
            {
                if !out.contains(head) {
                    out.push(*head);
                }
            }
            out.push(*row);
        }
    }
    out
}

/// Lay the filtered list out for a panel `cols` cells wide: one [`Row`] per
/// *display* line, so scroll positions and `max_scroll` count what is on
/// screen rather than what is in the table.
pub(crate) fn rows(needle: &str, cols: u16) -> Vec<Row> {
    let col = key_col(cols);
    // Two border columns, then the key column; the rest is the description.
    let width = (cols as usize).saturating_sub(2 + col).max(8);
    let mut out = Vec::new();
    for (k, d) in filtered(needle) {
        match (k, d) {
            ("", "") => out.push(Row::Spacer),
            ("", head) => out.push(Row::Head(head)),
            (k, d) => {
                let chars: Vec<char> = d.chars().collect();
                for (i, (a, b)) in wrap_indices(&chars, width).into_iter().enumerate() {
                    let text: String = chars[a..b].iter().collect();
                    match i {
                        0 => out.push(Row::Bind(k, text)),
                        _ => out.push(Row::Cont(text)),
                    }
                }
            }
        }
    }
    if out.is_empty() {
        out.push(Row::Note(format!("no binding matches \"{needle}\"")));
    }
    out
}

#[cfg(test)]
#[path = "helplayout_tests.rs"]
mod tests;
