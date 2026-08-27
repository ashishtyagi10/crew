//! Rung → `Vec<CardLine>`. Every format lands in the same representation the
//! chat cards use, so `render` is one mapper and each rung is tested as data.
//! Syntax colouring lives in `codepaint` and the opaque/metadata card in
//! `metacard` — both split out to keep this file under the length budget.
use crate::chatbody::{plain, CardLine};
use crate::viewpane::codepaint::{line_paint, CharPaint};
use crate::viewpane::detect::Format;
use crate::viewpane::load::{Loaded, MAX_VIEW_BYTES};
use crate::viewpane::metacard::opaque_card;
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
) -> Vec<CardLine> {
    let paints: Vec<Vec<CharPaint>> = text
        .split('\n')
        .map(|line| line_paint(line, lang, ink))
        .collect();
    painted(text, cols, &paints, ink, muted)
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
) -> Vec<CardLine> {
    let w = cols.saturating_sub(GUTTER_W).max(1);
    let mut out = Vec::new();
    let mut last = 0usize;
    let mut pos = 0usize;
    for (n, chars) in wrap(text, w) {
        let mut line: CardLine = if n == last {
            row(&" ".repeat(GUTTER_W), muted, false)
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
    out
}

/// The diff rung: a review rather than a colour per line. Pairing, word-level
/// marks and the header treatment live in [`super::diffpaint`]; this only lays
/// that paint down through the same numbered gutter every other rung uses.
fn diff_lines(text: &str, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    let paints = super::diffpaint::paint(text);
    painted(text, cols, &paints, t.ink, t.text_muted)
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
pub(crate) fn for_state(state: &LoadState, raw: bool, cols: usize) -> Vec<CardLine> {
    let t = crew_theme::theme();
    match state {
        LoadState::Loading { .. } => vec![banner("loading…", cols)],
        LoadState::Failed(msg) => vec![row(msg, t.ink, false)],
        LoadState::Ready { format, loaded } => ready_lines(*format, loaded, raw, cols),
    }
}

fn ready_lines(format: Format, loaded: &Loaded, raw: bool, cols: usize) -> Vec<CardLine> {
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
            numbered(text, cols, "", t.ink, t.text_muted)
        }
        Format::Diff => diff_lines(text, cols),
        Format::Markdown if !raw => super::mdrung::lines(text, cols),
        Format::Csv { delim } if !raw => super::csv::lines(text, delim, cols),
        // Fix 1: `Code`/`Data` used to reach here with no `lang`, which is
        // why every character painted the same `ink` regardless of what the
        // lexer would have called it. `format_lang` is `""` for every other
        // rung that lands here (raw `Markdown`/`Csv`), so their behaviour is
        // unchanged.
        _ => numbered(text, cols, format_lang(format), t.ink, t.text_muted),
    };
    out.extend(body);
    out
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
