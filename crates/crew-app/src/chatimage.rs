//! A picture named in a CHAT message. The chat pane paints no pictures — the
//! viewer does, on the paint layer under the rows `md::picture` reserves —
//! so in a card those twelve rows were twelve blank ones: a message with a
//! screenshot in it read as a message with a hole in it. Here the picture
//! becomes ONE muted row that says what it is, `[image] alt`, and every cell
//! of it carries the source as its link, so Cmd+click opens the file the way
//! it opens a link. Plain glyphs only: a colour bitmap in chrome is banned —
//! on a Nerd Font the tag is its picture icon (`glyphs`), still one cell.
use std::sync::Arc;

use crate::chatbody::{plain, CardCell, Color};
use crate::md::MdLine;

/// The cells of the one row a picture block becomes, unindented and unwrapped
/// — `chatmd` chunks them to the card's width like any other line.
pub(crate) fn cells(line: &MdLine, muted: Color) -> Vec<CardCell> {
    let src = crate::md::picture::src_of(line).unwrap_or_default();
    // A picture with no alt text is named by its source, so the row is never
    // just `[image]` with nothing to say which one.
    let alt = line
        .spans
        .first()
        .map(|s| s.text.trim())
        .filter(|a| !a.is_empty())
        .unwrap_or(src);
    let link: Option<Arc<str>> = (!src.is_empty()).then(|| Arc::from(src));
    format!("{} {alt}", crate::glyphs::pick(crate::glyphs::Glyph::Image))
        .chars()
        .map(|c| CardCell {
            link: link.clone(),
            ..plain(c, muted, false)
        })
        .collect()
}

#[cfg(test)]
#[path = "chatimage_tests.rs"]
mod tests;
