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
