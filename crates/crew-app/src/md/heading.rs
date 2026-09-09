//! Heading levels: stamped on the spans at layout, read back off a line by
//! the renderer. Every level is bold; the LEVEL is what the chat renderer
//! turns into an ink and, for the top one, a rule under the text — so the
//! level has to survive into the laid-out line rather than be folded into a
//! single "is a heading" bit the way it was.
use super::{MdLine, MdSpan};

/// Makes every span bold and stamps `level` on it.
pub(super) fn styled(level: u8, mut spans: Vec<MdSpan>) -> Vec<MdSpan> {
    for s in spans.iter_mut() {
        s.style.bold = true;
        s.style.heading = level;
    }
    spans
}

/// A heading's laid-out lines: every span bold and stamped, wrapped at the
/// width the chat card leaves it — an h1 gives up `chatheading::BADGE_W`
/// columns to the badge that leads its first row.
pub(super) fn lines(level: u8, spans: Vec<MdSpan>, cols: usize) -> Vec<MdLine> {
    let cols = crate::chatheading::wrap_cols(level, cols);
    super::layout::wrap_prose_lines(styled(level, spans), cols)
}

/// The heading level a laid-out line is, `0` for anything that is not one.
/// Read off the first AUTHORED span: a wrapped heading's every row carries
/// the level, and a quote bar prefixed in front of it does not.
pub(crate) fn level_of(line: &MdLine) -> u8 {
    line.spans
        .iter()
        .find(|s| !s.style.marker)
        .map_or(0, |s| s.style.heading)
}

#[cfg(test)]
#[path = "heading_tests.rs"]
mod tests;
