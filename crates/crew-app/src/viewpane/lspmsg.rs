//! What the language server actually SAID, on a row of its own under the line
//! it said it about.
//!
//! The viewer marked where the diagnostics were — a `●` in the margin, a curly
//! underline under the range — and never said what they were. A red dot beside
//! `unresolved_helper(d)` tells you there is something wrong with a line you
//! can already see is a line; the sentence rustc wrote ("cannot find function
//! `unresolved_helper` in this scope") is the part you opened the file for, and
//! it was on the wire, parsed, and thrown away at the margin.
//!
//! **Why a row rather than the end of the line.** The obvious place is after
//! the code, the way an editor's inline lens does it — and that is the one
//! place it does not fit. A line long enough to be wrong is a line with no
//! room left: at 107 columns the error above had fifteen to say fifty in, and
//! on a tile it had none at all. A compiler answers this by putting the
//! message under the code, and so does this: its own row, the full width of
//! the pane, pointing up at the line above it.
use crew_lsp::{Diagnostic, Severity};

use crate::chatbody::{plain, CardLine};
use crate::chatwidth::clip_w;
use crate::viewpane::lspdeco::{kind, Row};
use crate::viewpane::outline::Mark;

/// The glyph that points at the line the note is about.
const POINT: char = '\u{2191}';
/// How far the severity's colour is taken toward the page: enough that the
/// code reads first, not so far that the note reads as disabled.
const QUIET: f32 = 0.3;

/// The note for source line `n` (0-based): the worst diagnostic's first line,
/// plus how many others share the line.
pub(crate) fn note(diags: &[Diagnostic], n: usize) -> Option<(String, (u8, u8, u8))> {
    let mut here: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| (d.range.start.line as usize) == n)
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
        .collect();
    if here.is_empty() {
        return None;
    }
    // Errors before warnings, and among equals the one the server said first.
    here.sort_by_key(|d| u8::from(d.severity != Severity::Error));
    let worst = here[0];
    let (_, color) = crate::viewpane::lspgutter::mark_for(worst.severity)?;
    // A server's message may be a paragraph (rustc's notes, a type mismatch's
    // two halves). The row has one line, and it is the first one.
    let head = worst.message.lines().next().unwrap_or("").trim();
    let text = match here.len() {
        1 => head.to_string(),
        n => format!("{head}  +{}", n - 1),
    };
    Some((
        text,
        crate::anim::lerp_rgb(color, crew_theme::theme().page_bg, QUIET),
    ))
}

/// One note row: a blank gutter, then `↑ message` in the severity's quiet
/// colour, clipped to the text column's width.
///
/// The names a server quotes (rustc's ``cannot find function `helper` ``)
/// lose their ticks and stand upright in fuller ink: backticks are how a
/// plain-text message marks code, and a canvas that can draw the difference
/// was drawing the ticks instead.
fn row(text: &str, fg: (u8, u8, u8), gutter: usize, text_cols: usize) -> CardLine {
    let mut line: CardLine = " "
        .repeat(gutter)
        .chars()
        .map(|c| plain(c, fg, false))
        .collect();
    let body = spans(&format!("{POINT} {text}"));
    let flat: String = body.iter().map(|s| s.0).collect();
    let code_fg = crate::anim::lerp_rgb(fg, crew_theme::theme().ink, 0.5);
    let codes = body.iter().map(|s| s.1).chain(std::iter::repeat(false));
    for (c, code) in clip_w(&flat, text_cols.max(1)).chars().zip(codes) {
        let mut cell = plain(c, if code { code_fg } else { fg }, false);
        cell.italic = !code;
        line.push(cell);
    }
    line
}

/// `text` as `(char, in code)`, the backticks dropped. Unbalanced ticks are
/// left as written: a lone one is punctuation, not a span.
fn spans(text: &str) -> Vec<(char, bool)> {
    let balanced = text.matches('`').count() % 2 == 0;
    let mut code = false;
    let mut out = Vec::new();
    for c in text.chars() {
        if c == '`' && balanced {
            code = !code;
        } else {
            out.push((c, code));
        }
    }
    out
}

/// The server's word on the FILE, as a banner over it — the count it found,
/// that it is still starting, or why it never answered.
///
/// Only the document window said any of this: its legend has the room, and a
/// `/view` pane in the grid had margin marks with no total and — worse — no
/// way at all to explain a margin that never appeared, which is exactly what
/// `Lsp::Failed` is. A clean file still says nothing: the absence of marks is
/// the answer, and a pane does not spend a row on it.
pub(crate) fn top_banner(lsp: &crate::viewpane::lspjob::Lsp, cols: usize) -> Option<CardLine> {
    let say = match lsp {
        crate::viewpane::lspjob::Lsp::On(d) if !d.iter().any(marked) => return None,
        other => other.status()?,
    };
    Some(crate::viewpane::lines::banner(&say, cols))
}

/// Whether a diagnostic is one the margin marks — the two the summary counts.
fn marked(d: &Diagnostic) -> bool {
    matches!(d.severity, Severity::Error | Severity::Warning)
}

/// Put a note row under the last row of every flagged line, and move any
/// search marks below each insertion down with it.
///
/// Runs on the CACHED rows, before the blame and diagnostics margins prepend
/// their columns — so a note row is given the same blank margin every other
/// row has and the two columns stay aligned.
pub(crate) fn insert(
    lines: &mut Vec<CardLine>,
    marks: &mut [Mark],
    diags: &[Diagnostic],
    text_cols: usize,
) {
    if diags.is_empty() {
        return;
    }
    let gutter = crate::viewpane::linepaint::GUTTER_W;
    let mut cur: Option<usize> = None;
    let mut at = 0;
    while at < lines.len() {
        match kind(&lines[at], 0) {
            Row::Starts(n) => cur = Some(n),
            Row::Continues => {}
            Row::Other => cur = None,
        }
        let last_row = !lines
            .get(at + 1)
            .is_some_and(|l| matches!(kind(l, 0), Row::Continues));
        let note = cur
            .filter(|_| last_row)
            .and_then(|n| n.checked_sub(1))
            .and_then(|n| note(diags, n));
        at += 1;
        let Some((text, fg)) = note else { continue };
        lines.insert(at, row(&text, fg, gutter, text_cols.saturating_sub(gutter)));
        for m in marks.iter_mut().filter(|m| m.row >= at) {
            m.row += 1;
        }
        cur = None;
        at += 1;
    }
}

#[cfg(test)]
#[path = "lspmsg_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "lspbanner_tests.rs"]
mod banner_tests;

#[cfg(test)]
#[path = "lspcode_tests.rs"]
mod code_tests;
