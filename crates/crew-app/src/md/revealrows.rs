//! The revealed block's rows: its source lines in place of its render (see [`super::reveal`]).
//! Split from `reveal.rs`, which decides WHICH lines, for the line cap.
use super::reveal::{Kind, Reveal};
use super::{LineKind, MdLine, MdSpan, MdStyle};

/// `lines` with the rows of `r` replaced by its source lines, wrapped to `cols`.
///
/// The rows to replace are the ones holding any byte of `r` — plus, for a fence, the label row
/// above and the closing row below, which carry no bytes and are drawn by the source's own
/// fence lines instead. A byte keeps the style the render gave it, so a bold word stays bold
/// and a heading stays a heading; a byte the render never drew is a marker.
pub(crate) fn apply(mut lines: Vec<MdLine>, text: &str, r: Reveal, cols: usize) -> Vec<MdLine> {
    let (a, b) = (r.from as usize, (r.to as usize).min(text.len()));
    let inside = |l: &MdLine| {
        l.spans.iter().any(|s| {
            s.src
                .is_some_and(|at| (at as usize) <= b && at as usize + s.text.len() > a)
        })
    };
    let (Some(mut first), Some(mut last)) = (
        lines.iter().position(inside),
        lines.iter().rposition(inside),
    ) else {
        return lines;
    };
    if r.kind == Kind::Fence {
        if first > 0 && lines[first - 1].kind == LineKind::CodeHeader {
            first -= 1;
        }
        if lines
            .get(last + 1)
            .is_some_and(|l| l.kind == LineKind::CodeFooter)
        {
            last += 1;
        }
    }
    let styles = styles_of(&lines[first..=last], a, b);
    let raw = source_lines(text, a, b, &styles, r.kind, cols);
    lines.splice(first..=last, raw);
    lines
}

/// The style and link the render gave each byte of `[a, b)`, or `None` for a byte it never drew.
fn styles_of(rows: &[MdLine], a: usize, b: usize) -> Vec<Option<(MdStyle, Option<String>)>> {
    let mut out = vec![None; b.saturating_sub(a)];
    for s in rows.iter().flat_map(|l| &l.spans) {
        let Some(mut at) = s.src.map(|s| s as usize) else {
            continue;
        };
        for c in s.text.chars() {
            if let Some(slot) = at.checked_sub(a).and_then(|i| out.get_mut(i)) {
                *slot = Some((s.style, s.link.clone()));
            }
            at += c.len_utf8();
        }
    }
    out
}

fn source_lines(
    text: &str,
    a: usize,
    b: usize,
    styles: &[Option<(MdStyle, Option<String>)>],
    kind: Kind,
    cols: usize,
) -> Vec<MdLine> {
    let code_w = cols.saturating_sub(crate::chatfield::PAD * 2).max(1);
    let block = &text[a..b];
    let count = block.split('\n').count();
    let mut out = Vec::new();
    let mut lo = a;
    for (i, line) in block.split('\n').enumerate() {
        let line_kind = match kind {
            Kind::Prose => LineKind::Body,
            Kind::Indented => LineKind::Code,
            Kind::Fence if i == 0 => LineKind::CodeHeader,
            Kind::Fence if i + 1 == count && is_fence(line) => LineKind::CodeFooter,
            Kind::Fence => LineKind::Code,
        };
        let spans = line_spans(line, lo, a, styles, line_kind != LineKind::Body);
        let width = match line_kind {
            LineKind::Body => cols,
            _ => code_w,
        };
        let rows = match spans.is_empty() {
            true => vec![vec![super::layout::wrap::plain_span(String::new())]],
            false => super::layout::wrap::wrap_group(&spans, width),
        };
        out.extend(rows.into_iter().map(|spans| MdLine {
            spans,
            kind: line_kind,
        }));
        lo += line.len() + 1;
    }
    out
}

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

/// One source line as runs of one style each. A byte with no style of its own is a marker; on
/// a heading's line it is a heading's marker, so the line still reads as the heading it is.
fn line_spans(
    line: &str,
    lo: usize,
    a: usize,
    styles: &[Option<(MdStyle, Option<String>)>],
    code: bool,
) -> Vec<MdSpan> {
    let at = |i: usize| styles.get(lo + i - a).cloned().flatten();
    let heading = (0..line.len())
        .filter_map(at)
        .map(|(s, _)| s.heading)
        .find(|&h| h > 0)
        .unwrap_or(0);
    // On a code field a marker wears comment ink, not marker ink: the field reads leading
    // marker-ink cells as a quote's bar and starts itself after them (`chatfield`).
    let token = match code {
        true => super::syntax::Token::Comment,
        false => super::syntax::Token::Plain,
    };
    let marker = MdStyle {
        marker: true,
        heading,
        token,
        ..MdStyle::default()
    };
    let mut out: Vec<MdSpan> = Vec::new();
    for (i, c) in line.char_indices() {
        let (style, link) = at(i).unwrap_or((marker, None));
        match out.last_mut() {
            Some(s) if s.style == style && s.link == link => s.text.push(c),
            _ => out.push(MdSpan {
                text: c.to_string(),
                style,
                link,
                src: Some((lo + i) as u32),
            }),
        }
    }
    out
}

#[cfg(test)]
#[path = "revealrows_tests.rs"]
mod tests;
