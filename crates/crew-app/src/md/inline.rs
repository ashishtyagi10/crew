//! Inline-event folding shared by paragraph, heading, list-item and
//! table-cell contexts: turns a run of pulldown-cmark leaf/span events into
//! styled `MdSpan`s.
use pulldown_cmark::{Event, Tag, TagEnd};

use super::autolink::autolink;
use crate::md::{MdSpan, MdStyle};

/// Nesting state carried across one inline run (paragraph, heading, ...).
#[derive(Default)]
pub(super) struct InlineState {
    bold: u32,
    italic: u32,
    link: Vec<String>,
    // Whether a `SoftBreak` becomes a hard line-break (`chat`) or, per
    // CommonMark, a plain space joining the two sides (default `render`).
    keep_soft_breaks: bool,
}

impl InlineState {
    pub(super) fn new(keep_soft_breaks: bool) -> Self {
        Self {
            keep_soft_breaks,
            ..Default::default()
        }
    }

    fn style(&self) -> MdStyle {
        MdStyle {
            bold: self.bold > 0,
            italic: self.italic > 0,
            code: false,
            heading: 0,
        }
    }

    fn push_text(&self, spans: &mut Vec<MdSpan>, text: String, code: bool) {
        if text.is_empty() {
            return;
        }
        let mut style = self.style();
        style.code = code;
        spans.push(MdSpan {
            text,
            style,
            link: self.link.last().cloned(),
        });
    }
}

/// Applies one non-block-boundary event to `spans`, mutating `state`. Public
/// within `md::parse` so list-item folding (which mixes inline content with
/// nested block-level lists) can drive it one event at a time.
pub(super) fn apply_inline_event(
    event: Event<'_>,
    state: &mut InlineState,
    spans: &mut Vec<MdSpan>,
) {
    match event {
        Event::Text(t) => state.push_text(spans, t.into_string(), false),
        Event::Code(t) => state.push_text(spans, t.into_string(), true),
        // CommonMark joins a soft break with a single space; chat prose
        // (`keep_soft_breaks`) instead treats it as an intentional line
        // break, since users press Enter meaning "new line".
        Event::SoftBreak if state.keep_soft_breaks => spans.push(MdSpan {
            text: "\n".into(),
            style: MdStyle::default(),
            link: None,
        }),
        Event::SoftBreak => state.push_text(spans, " ".into(), false),
        Event::HardBreak => spans.push(MdSpan {
            text: "\n".into(),
            style: MdStyle::default(),
            link: None,
        }),
        Event::Start(Tag::Strong) => state.bold += 1,
        Event::End(TagEnd::Strong) => state.bold = state.bold.saturating_sub(1),
        Event::Start(Tag::Emphasis) | Event::Start(Tag::Strikethrough) => state.italic += 1,
        Event::End(TagEnd::Emphasis) | Event::End(TagEnd::Strikethrough) => {
            state.italic = state.italic.saturating_sub(1)
        }
        Event::Start(Tag::Link { dest_url, .. }) => state.link.push(dest_url.into_string()),
        Event::End(TagEnd::Link) => {
            state.link.pop();
        }
        Event::Html(t) | Event::InlineHtml(t) => state.push_text(spans, t.into_string(), false),
        _ => {}
    }
}

/// Consumes an already-open `List` — and everything nested inside it, no
/// matter how deep — without recursing, folding any text it contains into
/// `spans` at the current level. Used once block-nesting has hit its depth
/// cap (see `parse::MAX_NEST_DEPTH`) so pathological input can't grow the
/// call stack: an `open` counter tracks further `List` starts/ends instead.
pub(super) fn fold_nested_list<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    spans: &mut Vec<MdSpan>,
    keep_soft_breaks: bool,
) {
    let mut state = InlineState::new(keep_soft_breaks);
    let mut open = 1u32;
    loop {
        match events.next() {
            Some(Event::Start(Tag::List(_))) => open += 1,
            Some(Event::End(TagEnd::List(_))) => {
                open -= 1;
                if open == 0 {
                    break;
                }
            }
            Some(event) => apply_inline_event(event, &mut state, spans),
            None => break,
        }
    }
}

/// Consumes events until (and including) `stop`, folding them into styled
/// spans; bare URLs in the result become link spans.
pub(super) fn collect_inline<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    stop: TagEnd,
    keep_soft_breaks: bool,
) -> Vec<MdSpan> {
    let mut state = InlineState::new(keep_soft_breaks);
    let mut spans = Vec::new();
    loop {
        match events.next() {
            Some(Event::End(end)) if end == stop => break,
            Some(event) => apply_inline_event(event, &mut state, &mut spans),
            None => break,
        }
    }
    autolink(spans)
}
