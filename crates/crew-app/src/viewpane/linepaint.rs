//! Turning a file's text into painted rows: the line-number gutter, syntax
//! colours, the diff colouring, and the marks on trailing whitespace.
//!
//! Split from [`super::lines`] for the line cap, along the line between
//! deciding WHICH lines a viewer state shows and painting one.
pub(crate) use super::linemarks::*;
use crate::chatbody::{plain, CardLine};
use crate::viewpane::codepaint::{line_paint, CharPaint};

/// Width of the line-number gutter, digits plus one space.
pub(crate) const GUTTER_W: usize = 6;

pub(crate) fn row(s: &str, fg: (u8, u8, u8), bold: bool) -> CardLine {
    s.chars().map(|c| plain(c, fg, bold)).collect()
}

/// Hard-wrap `text` at `w` display columns, tagging each row with its 1-based
/// source line (continuations repeat it so the gutter can blank them).
pub(crate) fn wrap(text: &str, w: usize) -> Vec<(usize, Vec<char>)> {
    let mut out = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        let n = i + 1;
        let chars: Vec<char> = line.chars().collect();
        if w == 0 || chars.is_empty() {
            out.push((n, Vec::new()));
            continue;
        }
        let mut s = 0;
        while s < chars.len() {
            let e = crate::chatwidth::fit_end(&chars, s, w);
            out.push((n, chars[s..e].to_vec()));
            s = e;
        }
    }
    out
}

/// The paint for source line `n` (1-based), columns `[pos, pos + len)`.
/// `tokenize`'s losslessness — one paint entry per character, covering the
/// whole line — is enforced by tests, not the type system, so this is a
/// `.get` chain rather than a direct index: a regression there makes this
/// row fall back to uniform ink (see the `None` arm at the call site)
/// instead of panicking the winit thread mid-frame.
pub(crate) fn row_paint(
    paints: &[Vec<CharPaint>],
    n: usize,
    pos: usize,
    len: usize,
) -> Option<&[CharPaint]> {
    paints.get(n.wrapping_sub(1))?.get(pos..pos + len)
}

/// Numbered rows for the gutter rungs, syntax-coloured when `lang` names a
/// `md::syntax` language (Fix 1: `Code`/`Data` used to reach this function and
/// paint every character `ink`, so keywords, strings and comments were
/// indistinguishable from plain identifiers). `pos` tracks the char offset
/// into the CURRENT source line, resetting whenever `wrap` moves to the next
/// one: `wrap`'s rows for one line are emitted in left-to-right order with no
/// gaps, so a running counter is enough to slice that line's paint back out
/// per row without `wrap` itself needing to carry the offset.
pub(crate) fn numbered(
    text: &str,
    cols: usize,
    lang: &str,
    ink: (u8, u8, u8),
    muted: (u8, u8, u8),
    ws: &[Vec<bool>],
) -> Vec<CardLine> {
    let mut paints: Vec<Vec<CharPaint>> = text
        .split('\n')
        .map(|line| line_paint(line, lang, ink))
        .collect();
    // The tokenizer sees the expanded text and has no idea a run of spaces
    // used to be a tab — this is the only place that knows.
    super::whitespace::dim(&mut paints, ws, muted);
    painted(text, cols, &paints, ink, muted).0
}

/// The numbered-gutter body, given a paint per character. Shared by the
/// syntax rungs and the diff rung — they differ only in how the paint is
/// worked out, never in how it is laid down.
pub(crate) fn painted(
    text: &str,
    cols: usize,
    paints: &[Vec<CharPaint>],
    ink: (u8, u8, u8),
    muted: (u8, u8, u8),
) -> (Vec<CardLine>, Vec<usize>) {
    let w = cols.saturating_sub(GUTTER_W).max(1);
    let mut out = Vec::new();
    // Which source line each rendered row came from — how a landmark in the
    // text ([`super::outline`]) becomes a row to scroll to.
    let mut src = Vec::new();
    let mut last = 0usize;
    let mut pos = 0usize;
    for (n, chars) in wrap(text, w) {
        src.push(n - 1);
        let mut line: CardLine = if n == last {
            // A continuation says so. A blank gutter beside a wrapped line
            // and a blank gutter beside a genuinely empty numbered line look
            // identical, and in a wrapped file most rows are one or the
            // other.
            let mut cont = row(&" ".repeat(GUTTER_W), muted, false);
            if let Some(cell) = cont.get_mut(GUTTER_W - 2) {
                cell.c = '\u{21aa}';
            }
            cont
        } else {
            pos = 0;
            row(&format!("{n:>5} "), muted, false)
        };
        last = n;
        let row_paint = row_paint(paints, n, pos, chars.len());
        pos += chars.len();
        match row_paint {
            Some(paint) => line.extend(
                chars
                    .iter()
                    .zip(paint)
                    .map(|(c, (fg, bold))| plain(*c, *fg, *bold)),
            ),
            None => line.extend(chars.iter().map(|c| plain(*c, ink, false))),
        }
        out.push(line);
    }
    (out, src)
}

/// Rewrite the gutter of every row that STARTS a source line with `nums`,
/// right-aligned in [`GUTTER_W`]; a `None` leaves the gutter blank. Wrapped
/// continuations are left alone — they already carry the `\u{21aa}` that
/// distinguishes them from an empty numbered line.
pub(crate) fn renumber(
    lines: &mut [CardLine],
    src: &[usize],
    nums: &[Option<usize>],
    muted: (u8, u8, u8),
) {
    let mut last = usize::MAX;
    for (row, line) in lines.iter_mut().enumerate() {
        let n = src.get(row).copied().unwrap_or(0);
        let first = n != last;
        last = n;
        if !first {
            continue;
        }
        let text = match nums.get(n).copied().flatten() {
            Some(v) => format!("{v:>5} "),
            None => " ".repeat(GUTTER_W),
        };
        for (cell, c) in line.iter_mut().zip(text.chars()) {
            cell.c = c;
            cell.fg = muted;
        }
    }
}
