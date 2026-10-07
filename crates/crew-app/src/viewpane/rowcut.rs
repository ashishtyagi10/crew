//! What a wrapped row of the viewer keeps and drops at its edges: how far its
//! continuations hang, and a separator dot stranded at its end. Split from
//! [`super::linepaint`] (line cap).

/// How far a wrapped line's continuations hang: its own indentation, so a
/// wrapped statement stays under its block instead of starting flush at the
/// gutter — at most half the row, so a deep line still has room to wrap.
pub(super) fn hang(chars: &[char], w: usize) -> usize {
    chars
        .iter()
        .take_while(|c| c.is_whitespace())
        .count()
        .min(w / 2)
}

/// Blank a ` · ` separator the wrap left at the end of a row. The listings
/// join their parts with it (`/tools`, `/watching`), and a row ending in a
/// dot that separates it from nothing reads as a stray mark. Paint only: the
/// row keeps its length, so the offsets still partition the line, and a
/// selection copies from the source, dot included.
pub(super) fn strand_dot(row: &mut [char]) {
    let n = row.len();
    let lone = n >= 2 && row[n - 1] == ' ' && row[n - 2] == '\u{b7}';
    if lone && (n == 2 || row[n - 3] == ' ') {
        row[n - 2] = ' ';
    }
}
