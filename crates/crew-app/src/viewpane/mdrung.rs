//! The markdown rung: the render, full width, and nothing else. The
//! source|preview split the old `/md` pane drew is gone — showing markdown
//! source beside its render is a dev tool wearing a reading experience's
//! clothes, and `s` covers the times you genuinely need the bytes. The one exception is the
//! block an editing caret is in, which shows its own markers so they can be edited
//! ([`crate::md::reveal`]).
use crate::chatbody::CardLine;
use crate::viewpane::outline::Mark;

/// Rendered markdown for `cols` columns.
///
/// `chatmd::map_lines` prepends an unconditional one-column indent to every
/// line (it is shared with the chat card layout), so content is wrapped one
/// column narrower — without that, every width-filling row loses its last
/// column when `cells` draws at `cols`.
///
/// `reveal` is the block an editing caret is in, drawn as the markdown it is written in
/// ([`crate::md::reveal`]); `None` for a reader.
pub(crate) fn lines(
    text: &str,
    cols: usize,
    reveal: Option<crate::md::reveal::Reveal>,
) -> (Vec<CardLine>, Vec<Mark>, Vec<crate::chatmd::Picture>) {
    let fg = crew_theme::theme().ink;
    let content_w = cols.saturating_sub(1);
    let rendered = crate::md::render_revealed(text, content_w, reveal);
    // Headings are the landmarks `]` and `[` step between, and the renderer
    // has already decided which lines are headings — asking it beats
    // re-parsing the source and then having to map source lines back onto
    // wrapped rows.
    let marks = rendered
        .iter()
        .enumerate()
        .filter_map(|(row, line)| {
            let level = line
                .spans
                .first()
                .filter(|s| s.style.heading > 0)?
                .style
                .heading;
            // A revealed heading wears its `#`s as markers; the landmark is its words.
            let label: String = line
                .spans
                .iter()
                .filter(|s| !s.style.marker)
                .map(|s| s.text.as_str())
                .collect();
            Some(Mark {
                row,
                label: label.trim().to_string(),
                depth: level,
            })
        })
        .collect();
    let (lines, pics) = crate::chatmd::with_pictures(rendered, content_w, fg);
    (lines, marks, pics)
}

/// [`lines`] over the PREPARED text (tabs expanded, `\r` dropped), with the reveal taken from
/// the file's bytes into the prepared text's, and every cell's byte taken back again — the
/// caret splices into the file, not into what was drawn ([`super::srcmap`]).
pub(crate) fn rung(
    text: &str,
    map: &super::srcmap::SrcMap,
    cols: usize,
    reveal: Option<crate::md::reveal::Reveal>,
) -> (Vec<CardLine>, Vec<Mark>, Vec<crate::chatmd::Picture>) {
    let reveal = reveal.map(|r| crate::md::reveal::Reveal {
        from: map.to_prep(r.from),
        to: map.to_prep(r.to),
        ..r
    });
    let (mut out, marks, pics) = lines(text, cols, reveal);
    map.remap(&mut out);
    (out, marks, pics)
}

#[cfg(test)]
#[path = "mdrung_tests.rs"]
mod tests;
