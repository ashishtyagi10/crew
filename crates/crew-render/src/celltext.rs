//! Rich-text buffer builder for CellGrid.
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, Style, Weight, Wrap};

use crate::cellgrid::CellView;

/// Font metrics shared across all pane buffers.
pub(crate) struct FontParams {
    pub font_size: f32,
    pub line_height: f32,
    /// The fixed cell advance every glyph is snapped to (see [`cell_metrics`]).
    pub cell_w: f32,
    /// Chosen family name; `None`/empty falls back to the system monospace.
    pub family: Option<String>,
    /// Base weight for non-bold text (CSS scale; 400 normal, 500 medium).
    /// Light themes use 500 so ink reads crisp on a bright page; bold cells
    /// always shape at `Weight::BOLD` regardless.
    pub weight: u16,
}

/// The cosmic-text `Family` for an optional family name (empty/`None` → system monospace).
pub(crate) fn family_from(opt: &Option<String>) -> Family<'_> {
    match opt {
        Some(name) if !name.is_empty() => Family::Name(name),
        _ => Family::Monospace,
    }
}

/// Build a new `Buffer` for one pane's cells at the given cols/rows.
/// The buffer is sized to `(w, h)` pixels and laid out as a cols×rows grid.
pub(crate) fn build_pane_buffer(
    font_system: &mut FontSystem,
    cells: &[CellView],
    cols: usize,
    rows: usize,
    w: f32,
    h: f32,
    params: &FontParams,
) -> Buffer {
    let mut buffer = Buffer::new(
        font_system,
        Metrics::new(params.font_size, params.line_height),
    );
    buffer.set_wrap(font_system, Wrap::None);
    buffer.set_size(font_system, Some(w), Some(h));
    // Snap every glyph advance to the fixed cell box, so the grid — and every
    // box-drawing border in it — stays identical whatever family is chosen
    // (fallback glyphs, bold runs and wide CJK/emoji included).
    //
    // Unit quirk: without cosmic-text's `monospace_fallback` feature (not in
    // its defaults, so compiled out of our glyphon build), the only effect of
    // `monospace_width` is to round each advance to the nearest multiple of
    // `monospace_width / font_size`. Passing `cell_w * font_size` makes that
    // quantum exactly one cell, which is the snapping we want; passing the
    // intuitive `cell_w` yields a ~cell_w/font_size quantum — advances stay
    // at the font's natural width and the text grid drifts off the quad grid
    // (verified by `bold_glyphs_snap_to_the_same_cell_advance`).
    buffer.set_monospace_width(font_system, Some(params.cell_w * params.font_size));

    fill_rich_text(
        &mut buffer,
        font_system,
        cells,
        cols,
        rows,
        &params.family,
        params.weight,
    );
    buffer
}

/// Per-column styling key, used to coalesce horizontally-adjacent cells that
/// share a style into one shaping span. `Default` = an empty cell (rendered as a
/// space in the buffer's default attrs).
#[derive(PartialEq)]
enum RunKey {
    Default,
    Styled((u8, u8, u8), bool, bool),
}

/// Fill an existing `Buffer` with rich-text spans for `cells` laid out in cols×rows.
///
/// The whole grid is built into a single text `String`, and runs of adjacent
/// cells that share styling collapse into one span. This avoids the previous
/// one-`String`-and-one-span-per-cell layout (10k+ heap allocations per pane per
/// frame on a large grid), cutting both allocations and shaping spans sharply.
pub(crate) fn fill_rich_text(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    cells: &[CellView],
    cols: usize,
    rows: usize,
    family: &Option<String>,
    weight: u16,
) {
    let fam = family_from(family);
    let base = Weight(weight);
    // Bucket cells into a single flat rows×cols grid — one allocation per pane
    // per frame, instead of a Vec-of-Vecs (one inner Vec allocated per row).
    let mut grid: Vec<Option<&CellView>> = vec![None; rows * cols];
    for cell in cells {
        let r = cell.row as usize;
        let c = cell.col as usize;
        if r < rows && c < cols {
            grid[r * cols + c] = Some(cell);
        }
    }

    let default_attrs = Attrs::new().family(fam).weight(base);

    // Build the entire buffer text once, recording `(start, end, key)` byte
    // ranges into it; consecutive same-key cells extend the current run.
    let mut text = String::with_capacity(rows * (cols + 1));
    let mut runs: Vec<(usize, usize, RunKey)> = Vec::new();
    for row_i in 0..rows {
        for c in 0..cols {
            let (ch, key) = match grid[row_i * cols + c] {
                Some(cell) => (cell.c, RunKey::Styled(cell.fg, cell.bold, cell.italic)),
                None => (' ', RunKey::Default),
            };
            let start = text.len();
            text.push(ch);
            match runs.last_mut() {
                Some((_, last_end, last_key)) if *last_key == key => *last_end = text.len(),
                _ => runs.push((start, text.len(), key)),
            }
        }
        if row_i + 1 < rows {
            let start = text.len();
            text.push('\n');
            runs.push((start, text.len(), RunKey::Default));
        }
    }

    let spans: Vec<(&str, Attrs<'_>)> = runs
        .iter()
        .map(|(s, e, key)| {
            let attrs = match key {
                RunKey::Default => default_attrs.clone(),
                RunKey::Styled(fg, bold, italic) => {
                    let mut a = Attrs::new()
                        .family(fam)
                        .color(Color::rgb(fg.0, fg.1, fg.2))
                        .weight(if *bold { Weight::BOLD } else { base });
                    if *italic {
                        a = a.style(Style::Italic);
                    }
                    a
                }
            };
            (&text[*s..*e], attrs)
        })
        .collect();

    buffer.set_rich_text(font_system, spans, &default_attrs, Shaping::Advanced, None);
}

/// The fixed cell box for a font size: `(cell_w, cell_h)` =
/// `(0.6, 1.25) × font_size`, rounded to WHOLE pixels. Deliberately
/// independent of the font family — glyphs are snapped to this advance at
/// layout time (see [`build_pane_buffer`]) — so switching fonts never moves a
/// pane, a border, or the grid. `font_size` arrives in physical pixels, so
/// rounding puts every column, row, and glyph advance on a pixel boundary —
/// no half-pixel smear on the text or the box-drawing borders.
pub(crate) fn cell_metrics(font_size: f32) -> (f32, f32) {
    (
        (font_size * 0.6).round().max(1.0),
        (font_size * 1.25).round().max(1.0),
    )
}

#[cfg(test)]
#[path = "celltext_tests.rs"]
mod tests;
