//! The markdown rung: the render, full width, and nothing else. The
//! source|preview split the old `/md` pane drew is gone — showing markdown
//! source beside its render is a dev tool wearing a reading experience's
//! clothes, and `s` covers the times you genuinely need the bytes.
use crate::chatbody::CardLine;
use crate::viewpane::outline::Mark;

/// Rendered markdown for `cols` columns.
///
/// `chatmd::map_lines` prepends an unconditional one-column indent to every
/// line (it is shared with the chat card layout), so content is wrapped one
/// column narrower — without that, every width-filling row loses its last
/// column when `cells` draws at `cols`.
pub(crate) fn lines(text: &str, cols: usize) -> (Vec<CardLine>, Vec<Mark>) {
    let fg = crew_theme::theme().ink;
    let content_w = cols.saturating_sub(1);
    let rendered = crate::md::render(text, content_w);
    // Headings are the landmarks `]` and `[` step between, and the renderer
    // has already decided which lines are headings — asking it beats
    // re-parsing the source and then having to map source lines back onto
    // wrapped rows.
    let marks = rendered
        .iter()
        .enumerate()
        .filter_map(|(row, line)| {
            line.spans.first().filter(|s| s.style.heading > 0)?;
            let label: String = line.spans.iter().map(|s| s.text.as_str()).collect();
            Some(Mark {
                row,
                label: label.trim().to_string(),
            })
        })
        .collect();
    (crate::chatmd::map_lines(rendered, content_w, fg), marks)
}

#[cfg(test)]
#[path = "mdrung_tests.rs"]
mod tests;
