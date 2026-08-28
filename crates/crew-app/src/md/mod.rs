//! Markdown model shared by the parser and the layout/render pass. Kept
//! intentionally dumb: no wrapping, no color — just parsed structure and
//! inline styling.
mod layout;
mod parse;
pub(crate) mod syntax;
mod syntaxdiff;
mod tasklist;

/// Parses `text` and lays it out into wrapped, styled lines ready to draw at
/// `cols` columns. Never panics, regardless of input. CommonMark semantics:
/// a single line break (soft break) joins with a space — the right default
/// for a future file/document viewer, where source text is often hard-wrapped.
pub(crate) fn render(text: &str, cols: usize) -> Vec<MdLine> {
    layout::lines(parse::parse(text), cols)
}

/// Same as `render`, but for chat message bodies: a single line break stays
/// a line break, since in chat prose pressing Enter means "new line", not
/// CommonMark's "soft break, join with a space".
pub(crate) fn render_chat(text: &str, cols: usize) -> Vec<MdLine> {
    layout::lines(parse::parse_with(text, true), cols)
}

/// Table layout for non-markdown callers (the viewer's CSV rung): column
/// widths, padded cells and the header rule, over spans the caller builds.
pub(crate) fn table_lines(
    header: Vec<Vec<MdSpan>>,
    rows: Vec<Vec<Vec<MdSpan>>>,
    cols: usize,
) -> Vec<MdLine> {
    layout::table_lines(header, rows, cols)
}

/// Per-span inline styling. Independent bits so they can combine freely
/// (`**bold _italic_**` yields a span with both `bold` and `italic` set).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) struct MdStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,  // inline code span
    pub heading: u8, // 0 = body text, 1..=6 = heading level
    /// A structural marker glyph — a list bullet/ordinal or a blockquote bar
    /// — rather than authored content. The chat renderer colours markers
    /// separately from the text they introduce.
    pub marker: bool,
    /// What this run of a fenced code block is — comment, string, keyword —
    /// so the chat renderer can colour inside code rather than painting the
    /// whole block one colour. `Plain` everywhere outside a fence, except
    /// task-list items: a checked ✓ carries `Added` (green), its text
    /// `Comment` (dim) — see `tasklist`.
    pub token: syntax::Token,
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
pub(crate) enum LineKind {
    Body,
    Quote,      // a line of blockquote prose (bar + quoted text)
    CodeHeader, // the fence's language label; drawn on the code field
    Code,       // verbatim code content (chat draws it on code_bg)
    CodeFooter, // the field's closing blank row
    Rule,       // horizontal rule
    Blank,      // paragraph separator
}

/// One wrapped, drawable line of a rendered markdown document.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct MdLine {
    pub spans: Vec<MdSpan>,
    pub kind: LineKind,
}
