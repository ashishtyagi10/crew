//! Task-list (`- [ ]` / `- [x]`) support: carries the checkbox state from
//! pulldown-cmark's `TaskListMarker` event through the inline span stream to
//! the parsed `ListItem`, and styles the checkbox glyph and item text at
//! layout time. Split out of `parse.rs`/`layout.rs` to keep both under their
//! line budgets.
use super::parse::ListItem;
use super::syntax::Token;
use super::{MdSpan, MdStyle};

/// A never-rendered span carrying a task item's checkbox state through the
/// inline span stream — the same convention `fold::newline_marker` uses for
/// hard breaks. `extract` pulls it back out (and always strips it), so it can
/// never reach layout. `marker: true` keeps it unambiguous: the parser never
/// sets `marker` on authored text.
pub(super) fn sentinel(checked: bool) -> MdSpan {
    MdSpan {
        text: if checked { "[x]" } else { "[ ]" }.into(),
        style: MdStyle {
            marker: true,
            ..MdStyle::default()
        },
        link: None,
    }
}

fn is_sentinel(s: &MdSpan) -> bool {
    s.style.marker && (s.text == "[x]" || s.text == "[ ]")
}

/// Builds one `ListItem` from its collected spans, pulling the task sentinel
/// (if any) back off them.
pub(super) fn item(ordered_idx: Option<u64>, depth: u8, spans: Vec<MdSpan>) -> ListItem {
    let (task, spans) = extract(spans);
    ListItem {
        ordered_idx,
        depth,
        task,
        spans,
    }
}

/// Pulls the leading task sentinel off one item's collected spans:
/// `Some(checked)` plus the spans without it. Any stray sentinel deeper in
/// (possible only past `parse::MAX_NEST_DEPTH`, where lists fold flat) is
/// stripped too — its checkbox state drops along with the list structure.
fn extract(spans: Vec<MdSpan>) -> (Option<bool>, Vec<MdSpan>) {
    let task = spans
        .first()
        .filter(|s| is_sentinel(s))
        .map(|s| s.text == "[x]");
    (
        task,
        spans.into_iter().filter(|s| !is_sentinel(s)).collect(),
    )
}

/// Bullet glyphs by nesting depth, cycling past the end. One glyph at every
/// level leaves two spaces of indent as the only thing separating a sub-point
/// from the point above it, and an agent's plan is nested three deep by the
/// second paragraph. This is what every prose typographer and every other
/// markdown renderer does; the ordered levels already renumber from 1.
const BULLETS: [char; 3] = ['\u{2022}', '\u{25e6}', '\u{25aa}']; // • ◦ ▪

/// The item's lead glyph: a checkbox for a task item, the usual
/// bullet/ordinal otherwise. Same trailing space either way, so task and
/// plain items share one hanging-indent computation in `layout::list_lines`.
pub(super) fn bullet(task: Option<bool>, ordered_idx: Option<u64>, depth: u8) -> String {
    match (task, ordered_idx) {
        (Some(true), _) => "\u{2713} ".into(),  // ✓
        (Some(false), _) => "\u{2610} ".into(), // ☐
        (None, Some(n)) => format!("{n}. "),
        (None, None) => format!("{} ", BULLETS[depth as usize % BULLETS.len()]),
    }
}

/// The marker span opening the item's first line. A checked task's ✓ carries
/// `Token::Added`, the same slot diff-added lines draw in, so it greens; every
/// other lead glyph stays a plain marker.
pub(super) fn head_span(prefix: String, task: Option<bool>) -> MdSpan {
    let token = if task == Some(true) {
        Token::Added
    } else {
        Token::Plain
    };
    MdSpan {
        text: prefix,
        style: MdStyle {
            marker: true,
            token,
            ..MdStyle::default()
        },
        link: None,
    }
}

/// A checked item's text recedes: every span takes `Token::Comment`, the same
/// rung dimmed code comments sit on. Unchecked and plain items pass through.
pub(super) fn body_spans(mut spans: Vec<MdSpan>, task: Option<bool>) -> Vec<MdSpan> {
    if task == Some(true) {
        for s in spans.iter_mut() {
            s.style.token = Token::Comment;
        }
    }
    spans
}

#[cfg(test)]
#[path = "tasklist_tests.rs"]
mod tests;
