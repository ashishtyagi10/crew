//! A picture named in a CHAT message: painted when the card can, named
//! when it cannot.
//!
//! The viewer paints a document's pictures on the paint layer under the rows
//! `md::picture` reserves. A card used to paint none — twelve blank rows, a
//! message with a hole in it — so the picture became ONE muted row saying
//! what it is, the picture glyph then the alt, every cell carrying the source
//! as its link so Cmd+click opens the file the way it opens a link.
//!
//! That row stays, and is now the caption. When the source is a local file
//! (absolute, or relative to the pane's working directory — the cache reads
//! no remote image, see `imgcache`) the card puts the box above it: twelve
//! rows of blank cells each marked with its row of the box (`CardCell::pic`),
//! which `chatpicpaint` reads back off the PLACED lines to lay the decoded
//! picture into — however the card was folded, scrolled or glided, the rows
//! carry their own identity. While the worker is still reading, the caption
//! is followed by a muted `loading…`; a file that is not there, or not a
//! picture, is the caption alone, as before. Plain glyphs only: a colour
//! bitmap in chrome is banned — on a Nerd Font the tag is the picture icon.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::chatbody::{plain, CardCell, CardLine, Color};
use crate::md::MdLine;

/// Rows the painted box takes — the viewer's, so a picture is the same
/// height in a card as in a document.
pub(crate) const ROWS: u16 = crate::md::picture::ROWS;

/// What the card does with one picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fate {
    /// Decoded and kept: paint the box, caption under it.
    Painted(PathBuf),
    /// A worker is reading it: caption, then `loading…`.
    Loading,
    /// Not a local file, not there, or not a picture: the caption alone.
    Named,
}

/// Decides `src`'s fate for a card whose working directory is `cwd`. Asking
/// is what starts the read; never a stat, never a block (see
/// `imgcache::probe`).
pub(crate) fn fate(src: &str, cwd: Option<&Path>) -> Fate {
    let Some(path) = crate::imgcache::locate(src, cwd) else {
        return Fate::Named;
    };
    match crate::imgcache::probe(&path) {
        crate::imgcache::Probe::Ready => Fate::Painted(path),
        crate::imgcache::Probe::Loading => Fate::Loading,
        crate::imgcache::Probe::Failed => Fate::Named,
    }
}

/// The cells of the caption row, unindented and unwrapped — `chatmd`
/// chunks them to the card's width like any other line.
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

/// The `loading…` row under the caption while the worker is out.
pub(crate) fn loading_cells(muted: Color) -> Vec<CardCell> {
    "loading\u{2026}"
        .chars()
        .map(|c| plain(c, muted, false))
        .collect()
}

/// The [`ROWS`] rows of the box, each `width` blank cells past the indent,
/// every one marked with its row and carrying `src` as its link — so the
/// whole picture is the file, to a click, the way the caption is.
pub(crate) fn box_lines(src: &str, width: usize, fg: Color) -> Vec<CardLine> {
    let link: Arc<str> = Arc::from(src);
    (0..ROWS)
        .map(|i| {
            let cell = CardCell {
                link: Some(link.clone()),
                pic: Some(i),
                ..plain(' ', fg, false)
            };
            std::iter::once(plain(' ', fg, false))
                .chain(std::iter::repeat_n(cell, width.max(1)))
                .collect()
        })
        .collect()
}

/// Reads a box row back: `(row of the box, source)` when every cell of
/// `line` past the indent is a marked blank. A row that grew a suffix (the
/// compact clamp's ` … +N`) is no longer one — it must not pull a twelve-row
/// picture over the cards under it.
pub(crate) fn box_row(line: &CardLine) -> Option<(u16, &str)> {
    let first = line.get(1)?;
    let i = first.pic?;
    let src = first.link.as_deref()?;
    line[1..]
        .iter()
        .all(|c| c.pic == Some(i) && c.c == ' ')
        .then_some((i, src))
}

#[cfg(test)]
#[path = "chatimage_tests.rs"]
mod tests;
