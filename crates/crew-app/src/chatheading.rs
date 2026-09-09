//! The rule under a top-level heading in a chat card. An `h1` is the one
//! title a message has, and a title is underlined — as wide as its text, not
//! the card, so it reads as the heading's own rather than as a divider
//! between two messages. Muted, like every rule the card draws.
use crate::chatbody::{plain, CardLine, Color};

/// The rule row for the heading row `above`: one indent cell, then `─` for
/// every display column the heading's text covers.
pub(crate) fn rule_row(above: &CardLine, muted: Color) -> CardLine {
    let width: usize = above
        .iter()
        .skip(1)
        .map(|c| crate::chatwidth::char_w(c.c))
        .sum();
    std::iter::once(plain(' ', muted, false))
        .chain((0..width).map(|_| plain('\u{2500}', muted, false)))
        .collect()
}

#[cfg(test)]
#[path = "chatheading_tests.rs"]
mod tests;
