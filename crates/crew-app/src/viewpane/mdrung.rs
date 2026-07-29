//! The markdown rung: the render, full width, and nothing else. The
//! source|preview split the old `/md` pane drew is gone — showing markdown
//! source beside its render is a dev tool wearing a reading experience's
//! clothes, and `s` covers the times you genuinely need the bytes.
use crate::chatbody::CardLine;

/// Rendered markdown for `cols` columns.
///
/// `chatmd::map_lines` prepends an unconditional one-column indent to every
/// line (it is shared with the chat card layout), so content is wrapped one
/// column narrower — without that, every width-filling row loses its last
/// column when `cells` draws at `cols`.
pub(crate) fn lines(text: &str, cols: usize) -> Vec<CardLine> {
    let fg = crew_theme::theme().ink;
    let content_w = cols.saturating_sub(1);
    crate::chatmd::map_lines(crate::md::render(text, content_w), content_w, fg)
}

#[cfg(test)]
#[path = "mdrung_tests.rs"]
mod tests;
