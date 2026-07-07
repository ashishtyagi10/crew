//! Markdown model shared by the parser and the layout/render pass. Kept
//! intentionally dumb: no wrapping, no color — just parsed structure and
//! inline styling.
mod layout;
mod parse;

/// Parses `text` and lays it out into wrapped, styled lines ready to draw at
/// `cols` columns. Never panics, regardless of input.
pub(crate) fn render(text: &str, cols: usize) -> Vec<MdLine> {
    layout::lines(parse::parse(text), cols)
}

/// Per-span inline styling. Independent bits so they can combine freely
/// (`**bold _italic_**` yields a span with both `bold` and `italic` set).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) struct MdStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,  // inline code span
    pub heading: u8, // 0 = body text, 1..=6 = heading level
}

/// A run of text sharing one style, optionally linking to a URL.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct MdSpan {
    pub text: String,
    pub style: MdStyle,
    pub link: Option<String>, // absolute URL this span links to
}

/// What a rendered line represents, so the chat pane knows how to draw it
/// (code lines get a background, rules get a divider glyph, ...).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)] // constructed by md::layout in Task 2
pub(crate) enum LineKind {
    Body,
    CodeHeader, // "╭─ lang" chrome line (chat draws it muted, no bg)
    Code,       // verbatim code content (chat draws it on code_bg)
    CodeFooter, // "╰─"
    Rule,       // horizontal rule
    Blank,      // paragraph separator
}

/// One wrapped, drawable line of a rendered markdown document.
#[derive(Clone, PartialEq, Eq, Debug)]
#[allow(dead_code)] // constructed by md::layout in Task 2
pub(crate) struct MdLine {
    pub spans: Vec<MdSpan>,
    pub kind: LineKind,
}
