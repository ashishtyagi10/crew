//! A reply that opens with a framed block gets one blank row under its
//! sender header.
//!
//! The header is a filled `sender` badge over a gutter bar. A body that opens
//! with prose sits under it as its caption, which is the card's design. A
//! body that opens with a framed block put that frame on the very next pixel
//! row: an `# h1` stacked its `#` badge flush under the sender's, so the two
//! read as one stepped tab, and a `>` quote hung its rule off the gutter bar,
//! one bar with a kink that changed colour. Every other block in a reply
//! already gets a blank row before it; the first one now does too.

/// Whether `text` (a message's source) opens with an h1, a quote, a fenced
/// code block or a table: the blocks that draw a badge, a bar or a field.
pub(crate) fn framed(text: &str) -> bool {
    let first = text
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default()
        .trim_start();
    first.starts_with("# ")
        || first.starts_with('>')
        || first.starts_with("```")
        || first.starts_with("~~~")
        || first.starts_with('|')
}

#[cfg(test)]
#[path = "chatlead_tests.rs"]
mod tests;
