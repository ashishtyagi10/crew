//! `[^label]` footnotes: the reference in the prose and the definition it
//! points at.
//!
//! pulldown-cmark hands a definition back where the SOURCE put it — usually
//! the bottom, but anywhere is legal — and laying it out there would drop a
//! numbered aside into the middle of a paragraph run. So the layout pulls
//! every definition out and lays them all at the end, under one muted rule,
//! as `label. text`; the reference stays in the prose as a small muted
//! `[label]`. A reference no definition answers is left exactly as it is: the
//! reader sees the mark the writer made, and nothing panics.
use super::parse::Block;
use super::{LineKind, MdLine, MdSpan, MdStyle};

/// One definition: its label and the prose of its body.
pub(super) type Note = (String, Vec<MdSpan>);

/// The span a `[^label]` reference becomes.
pub(super) fn reference(label: &str) -> MdSpan {
    MdSpan {
        text: format!("[{label}]"),
        style: MdStyle {
            footnote: true,
            ..MdStyle::default()
        },
        link: None,
        src: None,
    }
}

/// Pulls the definitions out of `blocks`, in document order, leaving the
/// rest exactly as it was.
pub(super) fn split(blocks: Vec<Block>) -> (Vec<Block>, Vec<Note>) {
    let mut rest = Vec::with_capacity(blocks.len());
    let mut notes = Vec::new();
    for block in blocks {
        match block {
            Block::Footnote { label, body } => notes.push((label, body_spans(body))),
            other => rest.push(other),
        }
    }
    (rest, notes)
}

/// A definition's prose: its paragraphs, joined by hard breaks. Anything
/// that is not prose — a fence, a table — is dropped: a footnote is an
/// aside, not a document.
fn body_spans(body: Vec<Block>) -> Vec<MdSpan> {
    let mut spans = Vec::new();
    for block in body {
        let Block::Paragraph(p) = block else {
            continue;
        };
        if !spans.is_empty() {
            spans.push(span("\n".into(), false));
        }
        spans.extend(p);
    }
    spans
}

fn span(text: String, marker: bool) -> MdSpan {
    MdSpan {
        text,
        style: MdStyle {
            marker,
            ..MdStyle::default()
        },
        link: None,
        src: None,
    }
}

/// The trailing block: a muted rule, then one `label. text` entry per
/// definition wrapped under a hanging indent. Preceded by a blank row when
/// `after_content` — when there is something above it to separate from.
/// Empty when there are no definitions.
pub(super) fn lines(notes: Vec<Note>, cols: usize, after_content: bool) -> Vec<MdLine> {
    if notes.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if after_content {
        out.push(MdLine {
            spans: Vec::new(),
            kind: LineKind::Blank,
        });
    }
    out.push(MdLine {
        spans: vec![span("─".repeat(cols), false)],
        kind: LineKind::Rule,
    });
    for (label, spans) in notes {
        let prefix = format!("{label}. ");
        let width = prefix.chars().count();
        let avail = cols.saturating_sub(width).max(1);
        for (i, mut line) in super::layout::wrap_prose_lines(spans, avail)
            .into_iter()
            .enumerate()
        {
            let head = if i == 0 {
                span(prefix.clone(), true)
            } else {
                span(" ".repeat(width), false)
            };
            line.spans.insert(0, head);
            out.push(line);
        }
    }
    out
}

#[cfg(test)]
#[path = "footnote_tests.rs"]
mod tests;
