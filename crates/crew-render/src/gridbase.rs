//! One baseline for every row of a cell grid.
//!
//! cosmic-text centres each line on the tallest face its glyphs came from,
//! so a row holding one fallback glyph (`▣` shapes from a Nerd Font: the
//! primary face has no such character) sat a pixel higher or lower than its
//! neighbours. Text on it moved a pixel, and the drawn card strokes on it
//! broke: a synthesized `│` is cached once per cache key, and the key does
//! not know the row, so the stroke cut for one row's baseline was drawn on
//! the other's, a pixel off. A terminal keeps the baseline where the
//! primary face puts it on every row; glyphon is told to (vendored patch 6,
//! `TextArea::baseline`), and the drawn glyphs are cut for the same line.
use glyphon::cosmic_text::{fontdb, LayoutRun};
use glyphon::Buffer;

/// The baseline every row of `buf` is drawn on, below the row's top: the
/// one the rows set wholly in the faces its ASCII came from agree on most.
/// `None` when no row is (a buffer of nothing but fallback glyphs), which
/// leaves each row its own. A buffer of one line returns that line's.
pub(crate) fn baseline(buf: &Buffer) -> Option<f32> {
    let runs: Vec<LayoutRun<'_>> = buf.layout_runs().collect();
    let mut primary: Vec<fontdb::ID> = Vec::new();
    for run in &runs {
        for g in run.glyphs.iter() {
            let ascii = run.text[g.start..g.end].is_ascii();
            if ascii && !primary.contains(&g.font_id) {
                primary.push(g.font_id);
            }
        }
    }
    // Counted on the offset's bits: the rows that agree agree exactly, as
    // they were laid out from the same faces at the same size.
    let mut seen: Vec<(u32, usize)> = Vec::new();
    for run in &runs {
        if run.glyphs.is_empty() || run.glyphs.iter().any(|g| !primary.contains(&g.font_id)) {
            continue;
        }
        let at = (run.line_y - run.line_top).to_bits();
        match seen.iter_mut().find(|(b, _)| *b == at) {
            Some((_, n)) => *n += 1,
            None => seen.push((at, 1)),
        }
    }
    // The first of the most common, so a tie is decided by reading order.
    let mut best: Option<(u32, usize)> = None;
    for &(b, n) in &seen {
        if best.is_none_or(|(_, m)| n > m) {
            best = Some((b, n));
        }
    }
    best.map(|(b, _)| f32::from_bits(b))
}

/// The baseline `run` is drawn on, as glyphon draws it with `base` (what
/// [`baseline`] returned for its buffer) — where the drawn glyphs on it are
/// cut from.
pub(crate) fn line_y(run: &LayoutRun<'_>, base: Option<f32>) -> f32 {
    base.map_or(run.line_y, |b| run.line_top + b)
}

#[cfg(test)]
#[path = "gridbase_tests.rs"]
mod tests;
