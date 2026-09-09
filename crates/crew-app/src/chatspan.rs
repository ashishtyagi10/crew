//! The ink one prose span draws in — colour, weight, slant, strike, the code
//! field under it and the URL it carries — read off its `MdStyle` and the
//! kind of line it sits on. Split out of `chatmd`, which maps spans to cells
//! and had no room left to say what a strike or a heading LEVEL looks like.
use std::sync::Arc;

use crate::chatbody::Color;
use crate::chatink;
use crate::md::{LineKind, MdSpan};

/// Everything a cell takes from the span it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpanInk {
    pub fg: Color,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub bg: Option<Color>,
    pub link: Option<Arc<str>>,
}

impl SpanInk {
    fn flat(fg: Color) -> Self {
        Self {
            fg,
            bold: false,
            italic: false,
            strike: false,
            bg: None,
            link: None,
        }
    }
}

/// The renderer's decoration for a cell that is (or is not) struck: the one
/// rule a chat cell can wear, in the cell's own colour.
pub(crate) fn deco(strike: bool) -> crew_theme::deco::Deco {
    crew_theme::deco::Deco {
        strike,
        ..crew_theme::deco::Deco::NONE
    }
}

/// Headings by level. The top one takes the accent — already floored against
/// the page by `palette::accent` — so the one title a message has reads as
/// its title; the second is `ink` itself, separated by weight and its own
/// line; the rest step back to `text_muted`, so an outline reads at a glance.
pub(crate) fn heading_fg(level: u8) -> Color {
    let th = crew_theme::theme();
    match level {
        1 => crate::palette::accent(),
        2 => th.ink,
        _ => th.text_muted,
    }
}

/// The ink for `span` on a line of `kind`, over `fg` (the card's body colour)
/// and `muted` (the theme's `text_muted`).
pub(crate) fn style(span: &MdSpan, kind: LineKind, fg: Color, muted: Color) -> SpanInk {
    // Checked before `kind`: the quote bar is prefixed to EVERY line of a
    // quote, including the Code lines of a fenced block inside it, and a bar
    // drawn in code colour on a code tint would read as part of the code.
    // A marker carrying a token overrides the marker colour — that is a
    // checked task's ✓, which draws in the diff-added green (`Token::Added`).
    if span.style.marker {
        return SpanInk::flat(match span.style.token {
            crate::md::syntax::Token::Plain => chatink::marker_fg(),
            token => chatink::token_fg(token),
        });
    }
    match kind {
        LineKind::CodeHeader | LineKind::CodeFooter | LineKind::Rule => SpanInk::flat(muted),
        // Inside a fence the SPAN decides the colour, not the line: the
        // tokenizer split it into comment / string / keyword / plain runs at
        // layout time (see `md::layout::code_spans`). Keywords are marked by
        // weight, not by a colour of their own: a fourth colour would crowd
        // the ladder the other classes sit on, and weight works on a
        // single-phosphor screen where hue cannot.
        LineKind::Code => SpanInk {
            fg: chatink::token_fg(span.style.token),
            bold: span.style.token == crate::md::syntax::Token::Keyword,
            bg: Some(chatink::code_bg()),
            ..SpanInk::flat(fg)
        },
        // A picture's rows carry only the sentinel span, which is never
        // drawn — the picture itself is paint, laid under these rows.
        LineKind::Blank | LineKind::Picture { .. } => SpanInk::flat(fg),
        LineKind::Body => body(span, fg, muted),
        LineKind::Quote => body(span, chatink::quote_fg(), muted),
    }
}

/// Styles one prose span over `base` — the colour its plain text draws in.
/// Precedence, highest first: footnote mark, link, heading, inline code,
/// token, then `base`. A span can carry several at once (`# A [link](u)`),
/// so the order is what decides; it is checked top-down rather than
/// accumulated, so each branch states its whole result. A strike is applied
/// last, over whichever branch won: the rule goes through everything, and
/// everything but a link steps back to `muted` — what is crossed out is
/// not what the message says, and a link stays the colour a link is.
fn body(span: &MdSpan, base: Color, muted: Color) -> SpanInk {
    let style = span.style;
    // Inline code inside a link keeps the code tint, as it did before.
    let bg = style.code.then(chatink::code_bg);
    let mut ink = if style.footnote {
        SpanInk::flat(muted)
    } else if let Some(url) = &span.link {
        SpanInk {
            fg: chatink::link_color(),
            bold: true,
            italic: style.italic,
            bg,
            link: Some(Arc::from(url.as_str())),
            strike: false,
        }
    } else if style.heading >= 1 {
        SpanInk {
            fg: heading_fg(style.heading),
            bold: true,
            italic: style.italic,
            bg,
            ..SpanInk::flat(base)
        }
    } else if style.code {
        SpanInk {
            fg: chatink::code_fg(),
            bold: style.bold,
            italic: style.italic,
            bg,
            ..SpanInk::flat(base)
        }
    } else {
        // Prose carrying a token: a checked task item's text, dimmed to the
        // comment rung (`md::tasklist::body_spans`).
        let fg = match style.token {
            crate::md::syntax::Token::Plain => base,
            token => chatink::token_fg(token),
        };
        SpanInk {
            fg,
            bold: style.bold,
            italic: style.italic,
            ..SpanInk::flat(base)
        }
    };
    if style.strike {
        ink.strike = true;
        if ink.link.is_none() {
            ink.fg = muted;
        }
    }
    ink
}

#[cfg(test)]
#[path = "chatspan_tests.rs"]
mod tests;
