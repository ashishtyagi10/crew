//! The outline's recognisers for Python, which nests by indentation, and
//! Markdown, whose outline is its headings.
//!
//! Python's definitions start with `def`, `async def` or `class`, and each is
//! inside whichever came before it less indented. Two things fake an indent
//! and are passed over: a line inside a triple-quoted string (a docstring
//! that shows a `def` in an example is not one), and a line that continues
//! brackets opened above it (`)` at column 0, closing a long signature, would
//! otherwise end the function it opens).
use super::sysoutline::Item;
use super::sysoutlinesig::{clip, compact, head};

#[cfg(test)]
#[path = "sysoutlinepy_tests.rs"]
mod tests;

/// The Python definitions in `src`.
pub(super) fn python(src: &str) -> Vec<Item> {
    let lines: Vec<String> = src.lines().map(str::to_owned).collect();
    let mut items = Vec::new();
    // The indent of each `def` or `class` the current line is inside.
    let mut around: Vec<usize> = Vec::new();
    let mut quote: Option<&[u8]> = None;
    let mut brackets = 0i32;
    for (i, line) in lines.iter().enumerate() {
        let skip = quote.is_some() || brackets > 0;
        let (q, delta) = scan(line.as_bytes(), quote);
        quote = q;
        brackets = (brackets + delta).max(0);
        let t = line.trim_start();
        if skip || t.is_empty() || t.starts_with('#') {
            continue;
        }
        let indent = line.len() - t.len();
        while around.last().is_some_and(|&d| d >= indent) {
            around.pop();
        }
        let kw = t.strip_prefix("async ").unwrap_or(t).trim_start();
        let at = t.len() - kw.len();
        let text = if kw.starts_with("def ") {
            compact(&head(&lines[i..], at, &[":"]), at)
        } else if kw.starts_with("class ") {
            clip(&head(&lines[i..], at, &[":"]))
        } else {
            continue;
        };
        items.push(Item::at(i, around.len(), text));
        around.push(indent);
    }
    items
}

/// One line from `quote` (the triple quote of a string left open above):
/// the string still open after it, and the brackets it opens less those it
/// closes, strings and a `#` comment aside.
fn scan<'q>(b: &[u8], mut quote: Option<&'q [u8]>) -> (Option<&'q [u8]>, i32) {
    const TRIPLES: [&[u8]; 2] = [b"\"\"\"", b"'''"];
    let (mut i, mut delta) = (0, 0);
    while i < b.len() {
        if let Some(q) = quote {
            if b[i..].starts_with(q) {
                quote = None;
                i += 3;
            } else {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            continue;
        }
        match b[i] {
            b'#' => break,
            b'"' | b'\'' => match TRIPLES.into_iter().find(|t| b[i..].starts_with(t)) {
                Some(t) => {
                    quote = Some(t);
                    i += 3;
                }
                None => {
                    let close = b[i];
                    i += 1;
                    while i < b.len() && b[i] != close {
                        i += if b[i] == b'\\' { 2 } else { 1 };
                    }
                    i += 1;
                }
            },
            b'(' | b'[' | b'{' => {
                delta += 1;
                i += 1;
            }
            b')' | b']' | b'}' => {
                delta -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    (quote, delta)
}

/// The Markdown headings in `src`, nested by level, with the shallowest level
/// present at the left: a README that starts at `##` is not all indented.
/// A `#` line in a fenced or indented code block is a comment, not a heading.
pub(super) fn markdown(src: &str) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    let mut fence: Option<(u8, usize)> = None;
    for (i, line) in src.lines().enumerate() {
        let t = line.trim_start();
        if let Some((ch, run)) = fence {
            let closes = fence_of(t).is_some_and(|(c, r)| c == ch && r >= run);
            if closes && t.trim_start_matches(ch as char).trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if line.len() - t.len() > 3 {
            continue;
        }
        if let Some(f) = fence_of(t) {
            fence = Some(f);
            continue;
        }
        let level = t.bytes().take_while(|&b| b == b'#').count();
        if (1..=6).contains(&level) && t[level..].starts_with(' ') {
            items.push(Item::at(i, level, clip(t.trim_end())));
        }
    }
    let top = items.iter().map(|it| it.depth).min().unwrap_or(0);
    for it in &mut items {
        it.depth -= top;
    }
    items
}

/// The fence `t` starts with, as its character and length: three or more
/// backticks or tildes. A backtick run with another backtick after it is
/// inline code (```` ```` ```diff ```` ````), which CommonMark says no fence
/// opens with, and which README.md starts a line with.
fn fence_of(t: &str) -> Option<(u8, usize)> {
    let ch = *t.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let run = t.bytes().take_while(|&b| b == ch).count();
    (run >= 3 && !(ch == b'`' && t[run..].contains('`'))).then_some((ch, run))
}
