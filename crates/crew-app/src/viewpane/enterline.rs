//! What Enter writes: the markdown that continues the block the caret is in. Split from
//! `edit.rs` for the line cap; `ViewPane::newline` is the caller.

/// Whether byte `at`, on the line starting at `line`, is inside code — where a newline is a
/// newline: no blank line, no list marker, only the line's own indent, which is what every code
/// editor carries down. A fence's own lines are not code: Enter on the opening one is the
/// label's, and on the closing one it starts whatever comes after the block.
pub(super) fn in_code(src: &str, at: usize, line: usize) -> bool {
    crate::md::reveal::block_at(src, at as u32).is_some_and(|r| {
        let last = src[..r.to as usize].rfind('\n').map_or(0, |i| i + 1);
        match r.kind {
            crate::md::reveal::Kind::Prose => false,
            crate::md::reveal::Kind::Indented => true,
            crate::md::reveal::Kind::Fence => line > r.from as usize && line < last,
        }
    })
}

/// The markdown a new line has to start with to continue the block the caret
/// is in, or `None` for prose (which needs a blank line instead).
///
/// Read off the SOURCE line the caret is on — the one place the markers are
/// still visible, which is exactly why the buffer is the source.
pub(super) fn continuation(line: &str) -> Option<String> {
    let indent: String = line
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let rest = &line[indent.len()..];
    // A quote inside a list, a list inside a quote: take the markers in the
    // order they were written.
    if let Some(after) = rest.strip_prefix("> ") {
        let inner = continuation(after).unwrap_or_default();
        return Some(format!("{indent}> {inner}"));
    }
    for m in ["- ", "* ", "+ "] {
        if rest.starts_with(m) {
            return Some(format!("{indent}{m}"));
        }
    }
    // `1. ` / `12) ` — the next item is numbered one higher, because a list
    // whose every item says `1.` renders correctly and reads as a mistake in
    // the file.
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if !digits.is_empty() {
        let after = &rest[digits.len()..];
        if let Some(sep) = [". ", ") "].iter().find(|s| after.starts_with(**s)) {
            let n: u32 = digits.parse().unwrap_or(1);
            return Some(format!("{indent}{}{sep}", n + 1));
        }
    }
    None
}
