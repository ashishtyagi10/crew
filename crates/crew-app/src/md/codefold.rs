//! A fenced or indented code block, folded into its lines — and where each line starts in the
//! source, so a caret can stand in code (see `md::source`). Split from `fold.rs` for the line cap.
use pulldown_cmark::{CodeBlockKind, Event, TagEnd};

use super::Block;

pub(super) fn collect_code_block<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    kind: CodeBlockKind,
) -> Block {
    let lang = match kind {
        CodeBlockKind::Fenced(l) => l.into_string(),
        CodeBlockKind::Indented => String::new(),
    };
    let (mut lines, mut starts) = (Vec::new(), Vec::new());
    let mut line = String::new();
    // Where the line being built starts in the source — `None` once any of it is not a
    // verbatim copy (see `md::source`), which is then a line the caret cannot stand in.
    let mut start: Option<Option<u32>> = None;
    loop {
        match events.next() {
            Some(Event::Text(t)) => {
                let base = crate::md::source::offset_for(&t);
                let mut at = 0usize;
                for piece in t.split_inclusive('\n') {
                    let here = base.map(|b| b + at as u32);
                    let body = piece.strip_suffix('\n').unwrap_or(piece);
                    start = Some(match start {
                        None => here,
                        // A line split across two events is one line only if they touch.
                        Some(s) => s.filter(|s| here == Some(s + line.len() as u32)),
                    });
                    line.push_str(body);
                    at += piece.len();
                    if piece.ends_with('\n') {
                        lines.push(std::mem::take(&mut line));
                        starts.push(start.take().flatten());
                    }
                }
            }
            Some(Event::End(TagEnd::CodeBlock)) | None => break,
            _ => {}
        }
    }
    if !line.is_empty() {
        lines.push(line);
        starts.push(start.flatten());
    }
    Block::CodeBlock {
        lang,
        lines,
        starts,
    }
}

#[cfg(test)]
#[path = "codefold_tests.rs"]
mod tests;
