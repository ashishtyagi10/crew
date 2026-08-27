//! Laying the split review out: two half-width columns of text with the
//! divider between them. The row MODEL — what pairs with what, and which line
//! number each side is on — is [`super::diffsplit`]; this is only how it is
//! drawn.
use crate::chatbody::{plain, CardLine};
use crate::viewpane::codepaint::CharPaint;
use crate::viewpane::diffpaint::{self, Kind};
use crate::viewpane::diffsplit::{rows, Row, DIVIDER, MIN_COLS};
use crate::viewpane::lines::GUTTER_W;
use crate::viewpane::outline::Mark;

/// The paint for one side of a pair, given the line it is paired with.
///
/// The refinement is [`diffpaint::refine`]'s, on exactly the same terms: the
/// leading `+`/`-` is not part of the text, two lines that differ almost
/// everywhere are left alone, and a side with no partner is drawn whole. The
/// unified rung reaches this through `diffpaint::paint`; here the pairing is
/// already known, so it is asked directly.
fn side_paint(text: &str, other: Option<&str>, kind: Kind) -> Vec<CharPaint> {
    let t = crew_theme::theme();
    let base = match kind {
        Kind::Added => t.ansi[2],
        Kind::Removed => t.ansi[1],
        _ => t.ink,
    };
    let body = &text[text.len().min(1)..];
    let mark = other
        .filter(|_| matches!(kind, Kind::Added | Kind::Removed))
        .and_then(|o| diffpaint::refine(&o[o.len().min(1)..], body))
        .map(|(_, mine)| mine);
    let Some((a, b)) = mark else {
        return body.chars().map(|_| (base, false)).collect();
    };
    let quiet = crate::anim::lerp_rgb(base, t.page_bg, DIM);
    body.chars()
        .enumerate()
        .map(|(i, _)| match (a..b).contains(&i) {
            true => (base, true),
            false => (quiet, false),
        })
        .collect()
}

/// How far a shared run recedes toward the page — the unified rung's own.
const DIM: f32 = 0.45;

/// One side's rows: the gutter (its own line number, or a `↪` continuation)
/// followed by `w` columns of text, wrapped. Always `w + GUTTER_W` wide, on
/// every row, so the divider between the halves is a straight line.
fn side_rows(
    no: Option<usize>,
    text: Option<&str>,
    other: Option<&str>,
    w: usize,
) -> Vec<CardLine> {
    let t = crew_theme::theme();
    let Some(text) = text else {
        // A side with no line here is blank — this is where one version of
        // the file simply has nothing, and drawing anything would invent it.
        return vec![row_of(&" ".repeat(w + GUTTER_W), t.text_muted)];
    };
    let kind = diffpaint::kind_of(text);
    let paint = side_paint(text, other, kind);
    let chars: Vec<char> = text.chars().skip(1).collect();
    let marker = text.chars().next().unwrap_or(' ');
    let mut out = Vec::new();
    let mut start = 0;
    let mut first = true;
    loop {
        let end = crate::chatwidth::fit_end(&chars, start, w.saturating_sub(1)).max(start + 1);
        let end = end.min(chars.len());
        let mut line = match first {
            true => gutter(no, t.text_muted),
            false => continuation(t.text_muted),
        };
        // The `+`/`-`/` ` marker leads the text and only its first row: a
        // continuation is the same line, and a second marker would read as a
        // second change.
        let base = paint.first().map_or(t.ink, |p| p.0);
        line.push(plain(if first { marker } else { ' ' }, base, false));
        for (i, c) in chars.iter().enumerate().take(end).skip(start) {
            let (fg, bold) = paint.get(i).copied().unwrap_or((t.ink, false));
            line.push(plain(*c, fg, bold));
        }
        while line.len() < w + GUTTER_W {
            line.push(plain(' ', t.text_muted, false));
        }
        out.push(line);
        first = false;
        start = end;
        if start >= chars.len() {
            break;
        }
    }
    out
}

fn row_of(s: &str, fg: (u8, u8, u8)) -> CardLine {
    s.chars().map(|c| plain(c, fg, false)).collect()
}

/// `   12 ` — the line's number in its own side of the file.
fn gutter(no: Option<usize>, fg: (u8, u8, u8)) -> CardLine {
    match no {
        Some(n) => row_of(&format!("{n:>width$} ", width = GUTTER_W - 1), fg),
        None => row_of(&" ".repeat(GUTTER_W), fg),
    }
}

/// A wrapped row says so, with the same `↪` the other rungs use.
fn continuation(fg: (u8, u8, u8)) -> CardLine {
    let mut v = row_of(&" ".repeat(GUTTER_W), fg);
    if let Some(cell) = v.get_mut(GUTTER_W - 2) {
        cell.c = '\u{21aa}';
    }
    v
}

/// The whole split rendering for `text` at `cols` columns, plus the landmarks
/// `]` / `[` step. `None` when the pane is too narrow to hold two honest
/// columns — the caller falls back to the unified rung.
pub(crate) fn lines(text: &str, cols: usize) -> Option<(Vec<CardLine>, Vec<Mark>)> {
    if cols < MIN_COLS {
        return None;
    }
    let t = crew_theme::theme();
    let half = (cols - 1) / 2;
    let w = half - GUTTER_W;
    let src_marks = super::outline::diff_marks(text);
    let mut out: Vec<CardLine> = Vec::new();
    let mut marks = Vec::new();
    let mut src = 0usize;
    for r in rows(text) {
        // Landmarks are found in the SOURCE and reported as rendered rows.
        if let Some((_, label)) = src_marks.iter().find(|(l, _)| *l == src) {
            marks.push(Mark {
                row: out.len(),
                label: label.clone(),
            });
        }
        match r {
            Row::Full(line, kind) => {
                let paint = diffpaint::header_paint(line, kind);
                let mut row: CardLine = line
                    .chars()
                    .zip(&paint)
                    .map(|(c, (fg, bold))| plain(c, *fg, *bold))
                    .collect();
                row.truncate(cols);
                out.push(row);
                src += 1;
            }
            Row::Pair(lno, l, rno, rt) => {
                let left = side_rows(lno, l, rt, w);
                let right = side_rows(rno, rt, l, w);
                let h = left.len().max(right.len());
                let blank = || row_of(&" ".repeat(w + GUTTER_W), t.text_muted);
                for i in 0..h {
                    let mut row = left.get(i).cloned().unwrap_or_else(blank);
                    row.push(plain(DIVIDER, t.legend_off, false));
                    row.extend(right.get(i).cloned().unwrap_or_else(blank));
                    out.push(row);
                }
                // A context line is ONE source line shown twice; a pair is
                // two. Either way the source cursor has to keep up with what
                // `rows` consumed, or the landmarks land on the wrong row.
                src += usize::from(l.is_some()) + usize::from(rt.is_some());
                if l == rt {
                    src -= 1; // context: the same line on both sides
                }
            }
        }
    }
    Some((out, marks))
}

#[cfg(test)]
#[path = "diffsplitdraw_tests.rs"]
mod tests;
