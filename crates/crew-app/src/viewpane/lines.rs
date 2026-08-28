//! Rung → `Vec<CardLine>`. Every format lands in the same representation the
//! chat cards use, so `render` is one mapper and each rung is tested as data.
//! Syntax colouring lives in `codepaint` and the opaque/metadata card in
//! `metacard` — both split out to keep this file under the length budget.
use crate::chatbody::{plain, CardLine};
use crate::viewpane::codepaint::{line_paint, CharPaint};
use crate::viewpane::detect::Format;
use crate::viewpane::load::{Loaded, MAX_VIEW_BYTES};
use crate::viewpane::metacard::opaque_card;
use crate::viewpane::outline::Mark;
use crate::viewpane::rendercap::{cap_render_lines, MAX_RENDER_LINES};
use crate::viewpane::LoadState;

/// Width of the line-number gutter, digits plus one space.
pub(crate) const GUTTER_W: usize = 6;

fn row(s: &str, fg: (u8, u8, u8), bold: bool) -> CardLine {
    s.chars().map(|c| plain(c, fg, bold)).collect()
}

/// Hard-wrap `text` at `w` display columns, tagging each row with its 1-based
/// source line (continuations repeat it so the gutter can blank them).
fn wrap(text: &str, w: usize) -> Vec<(usize, Vec<char>)> {
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
fn row_paint(paints: &[Vec<CharPaint>], n: usize, pos: usize, len: usize) -> Option<&[CharPaint]> {
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
fn numbered(
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
fn painted(
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
fn renumber(lines: &mut [CardLine], src: &[usize], nums: &[Option<usize>], muted: (u8, u8, u8)) {
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

/// Trailing whitespace on an ADDED line, shown as middle dots.
///
/// It is the review nit every diff tool marks, because it is invisible by
/// construction: the reviewer cannot see it and the author did not mean it.
/// Only added lines — what a removed line trailed with is not news — and only
/// past the marker column, so a line of pure indentation still reads as one.
fn mark_trailing_space(line: &mut CardLine, added: bool) {
    if !added {
        return;
    }
    let fg = crew_theme::theme().bell;
    let start = line
        .iter()
        .rposition(|c| !c.c.is_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    // `GUTTER_W + 1`: the gutter and the `+` marker are not the line's text.
    for cell in line.iter_mut().skip(start.max(GUTTER_W + 1)) {
        if cell.c == ' ' {
            cell.c = '\u{b7}';
            cell.fg = fg;
        }
    }
}

/// The diff rung: a review rather than a colour per line. Pairing, word-level
/// marks and the header treatment live in [`super::diffpaint`]; this only lays
/// that paint down through the same numbered gutter every other rung uses.
fn diff_lines(text: &str, cols: usize, ws: &[Vec<bool>]) -> (Vec<CardLine>, Vec<Mark>) {
    let t = crew_theme::theme();
    let mut paints = super::diffpaint::paint(text);
    super::whitespace::dim(&mut paints, ws, t.text_muted);
    let (mut lines, src) = painted(text, cols, &paints, t.ink, t.text_muted);
    // The gutter says where in the SOURCE you are, not where in the patch —
    // the same numbers the side-by-side rung has always shown (`diffnums`).
    renumber(
        &mut lines,
        &src,
        &super::diffnums::numbers(text),
        t.text_muted,
    );
    // Only the row a source line STARTS on carries its marker, so only that
    // row can be an added line whose tail is worth marking.
    let kinds: Vec<super::diffpaint::Kind> =
        text.split('\n').map(super::diffpaint::kind_of).collect();
    let mut last = usize::MAX;
    for (row, line) in lines.iter_mut().enumerate() {
        let n = src.get(row).copied().unwrap_or(0);
        let first = n != last;
        last = n;
        let added = first && kinds.get(n) == Some(&super::diffpaint::Kind::Added);
        mark_trailing_space(line, added);
    }
    // Landmarks are found in the source and reported as ROWS: a wrapped line
    // occupies several, and `]` has to land on the first of them.
    let marks = super::outline::diff_marks(text)
        .into_iter()
        .filter_map(|(line, label)| {
            let row = src.iter().position(|&s| s == line)?;
            Some(Mark { row, label })
        })
        .collect();
    (lines, marks)
}

fn banner(msg: &str, cols: usize) -> CardLine {
    let t = crew_theme::theme();
    let mut s: String = msg.chars().take(cols.max(1)).collect();
    while s.chars().count() < cols {
        s.push(' ');
    }
    row(&s, t.text_muted, false)
}

fn mb(bytes: u64) -> u64 {
    bytes / (1024 * 1024)
}

/// Lines for the pane's current state at `cols` columns. `raw` shows text
/// unrendered (the `s` toggle); it changes the `Markdown` and `Csv` rungs —
/// the two whose default rendering shows something OTHER than the bytes
/// themselves — and leaves every other rung alone, since those already show
/// the bytes as they are.
pub(crate) fn for_state(
    state: &LoadState,
    raw: bool,
    cols: usize,
    invisibles: bool,
    split: bool,
) -> (Vec<CardLine>, Vec<Mark>) {
    let t = crew_theme::theme();
    match state {
        LoadState::Loading { .. } => (vec![banner("loading…", cols)], Vec::new()),
        LoadState::Failed(msg) => (vec![row(msg, t.ink, false)], Vec::new()),
        LoadState::Ready { format, loaded } => {
            ready_lines(*format, loaded, raw, cols, invisibles, split)
        }
    }
}

/// Landmarks moved down past the banners drawn above the body.
fn shifted(marks: Vec<Mark>, above: usize) -> Vec<Mark> {
    marks
        .into_iter()
        .map(|m| Mark {
            row: m.row + above,
            ..m
        })
        .collect()
}

fn ready_lines(
    format: Format,
    loaded: &Loaded,
    raw: bool,
    cols: usize,
    invisibles: bool,
    split: bool,
) -> (Vec<CardLine>, Vec<Mark>) {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    if let Some(real) = loaded.truncated {
        out.push(banner(
            &format!(
                "showing first {} MB of {} MB — press o to open externally",
                mb(MAX_VIEW_BYTES),
                mb(real)
            ),
            cols,
        ));
    }
    // Fix 5: cap SOURCE lines before any rung renders them, not the RESULT
    // afterward — truncating the output would still pay the full render
    // cost this cap exists to avoid.
    let (text, capped_from) = cap_render_lines(&loaded.text);
    if let Some(real_lines) = capped_from {
        // `real_lines` counts lines in `loaded.text`, which is itself the 8
        // MB VIEW cap's slice whenever `loaded.truncated` is `Some` — an
        // exact count would then be naming the slice's line count as if it
        // were the file's, understating a file that runs on past the byte
        // cap. "at least" keeps the claim true either way.
        let of_lines = if loaded.truncated.is_some() {
            format!("at least {real_lines}")
        } else {
            real_lines.to_string()
        };
        out.push(banner(
            &format!(
                "showing first {MAX_RENDER_LINES} of {of_lines} lines — press o to open externally"
            ),
            cols,
        ));
    }
    // Tabs are expanded HERE, before any rung sees the text, so the syntax
    // paint, the wrap, the search and the diff pairing all agree about which
    // column a character is in. See `super::whitespace` — a tab has zero
    // display width, so an unexpanded one was not merely misaligned, it was
    // dropped, and a tab-indented file drew with no indentation at all.
    let prepared = super::whitespace::prepare(text, invisibles);
    let (text, ws) = (prepared.text.as_str(), prepared.marks.as_slice());
    let mut marks = Vec::new();
    let body = match format {
        Format::Opaque { why } => opaque_card(why, loaded.meta.as_ref(), cols),
        Format::Extract { via } => {
            out.push(banner(
                &format!(
                    "text extract via {} — press o to open the real file",
                    via.bin()
                ),
                cols,
            ));
            // An extract has no `md::syntax` language of its own — it is
            // prose lifted out of a PDF or a Word doc, not source.
            numbered(text, cols, "", t.ink, t.text_muted, ws)
        }
        // A pane too narrow for two honest columns answers `None` and the
        // unified rung takes it — the toggle is a request, not a promise the
        // width can always keep.
        Format::Diff => {
            let (lines, found) = match split
                .then(|| super::diffsplitdraw::lines(text, cols))
                .flatten()
            {
                Some(pair) => pair,
                None => diff_lines(text, cols, ws),
            };
            marks = shifted(found, out.len());
            lines
        }
        Format::Markdown if !raw => {
            let (lines, found) = super::mdrung::lines(text, cols);
            marks = shifted(found, out.len());
            lines
        }
        Format::Csv { delim } if !raw => super::csv::lines(text, delim, cols),
        // Fix 1: `Code`/`Data` used to reach here with no `lang`, which is
        // why every character painted the same `ink` regardless of what the
        // lexer would have called it. `format_lang` is `""` for every other
        // rung that lands here (raw `Markdown`/`Csv`), so their behaviour is
        // unchanged.
        _ => numbered(text, cols, format_lang(format), t.ink, t.text_muted, ws),
    };
    // A file with nothing in it renders as an empty pane, which is
    // indistinguishable from one that failed to render — and from one still
    // loading, since the "loading…" banner is gone by then. The opaque rung
    // is exempt: it draws a card ABOUT a file rather than the file, and has
    // plenty to say with no text at all.
    if text.trim().is_empty() && !matches!(format, Format::Opaque { .. }) {
        out.push(banner("this file is empty", cols));
        return (out, marks);
    }
    out.extend(body);
    (out, marks)
}

/// The `md::syntax` language tag for a rung, `""` when it has none. Only
/// `Code`/`Data` carry one — everything else that reaches `numbered` (an
/// `Extract`, or `Markdown`/`Csv` shown raw) is plain text.
fn format_lang(format: Format) -> &'static str {
    match format {
        Format::Code { lang } | Format::Data { lang } => lang,
        _ => "",
    }
}

#[cfg(test)]
#[path = "lines_tests.rs"]
mod tests;
