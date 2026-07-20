//! Rich-text buffer builder for CellGrid.
use std::cell::RefCell;
use std::collections::HashMap;

use glyphon::cosmic_text::fontdb;
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, Style, Weight, Wrap};
use unicode_width::UnicodeWidthChar;

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

/// Base text weight for every theme: Medium (500), so ink reads crisp and
/// substantial on both the bright paper pages and the dark newspaper pages.
/// (Dark pages used to keep Normal 400 — they now match light for a heavier,
/// more legible body.) `dark` is retained for the signature/hash key and a
/// possible future per-appearance split.
pub(crate) fn base_weight(_dark: bool) -> u16 {
    500
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

    fill_rich_text(&mut buffer, font_system, cells, cols, rows, params);
    // First sighting of a misbehaving glyph: measure it from the shaped
    // layout, cache its correction, and rebuild this buffer with it applied.
    // Steady-state frames hit the cache inside `fill_rich_text` directly.
    if detect_corrections(&buffer, font_system, params) {
        fill_rich_text(&mut buffer, font_system, cells, cols, rows, params);
    }
    buffer
}

/// Extra letter-spacing (em units, added pre-rounding at shape time) that
/// lands a width-1 glyph's advance exactly on one cell, or `None` when the
/// natural advance already rounds there. Needed because the monospace
/// rounding snaps to the NEAREST cell multiple: a glyph narrower than half a
/// cell (ComicMono Nerd Font Mono's `·`) rounds to a ZERO advance and every
/// glyph after it drifts one cell left; one wider than 1.5 cells rounds to
/// two and drifts the row right. Non-finite advances (GB18030 Bitmap CJK
/// quirk) are left alone.
fn cell_correction_em(a_em: f32, cell_em: f32) -> Option<f32> {
    if !a_em.is_finite() || (a_em / cell_em).round() as i64 == 1 {
        None
    } else {
        Some(cell_em - a_em)
    }
}

thread_local! {
    /// `(family, weight, char) → letter-spacing correction` (em units;
    /// `None` = the glyph behaves, cached so it isn't re-measured). Filled by
    /// `detect_corrections` from real shaped layouts — the char may resolve
    /// through font FALLBACK (ComicMono Nerd Font Mono lacks `·` entirely,
    /// so it shapes from a proportional fallback face), which no query
    /// against the primary family can predict. Advances are a font property,
    /// independent of size, so the cache survives font-size changes.
    static CORRECTION_CACHE: RefCell<HashMap<(String, u16, char), Option<f32>>> =
        RefCell::new(HashMap::new());
}

/// Whether a cell char is even eligible for correction: ASCII always shares
/// the face's base advance (that's what fixed-pitch means), and wide glyphs
/// keep their existing two-cell behavior — only width-1 symbols qualify.
fn correctable(c: char) -> bool {
    !c.is_ascii() && UnicodeWidthChar::width(c) == Some(1)
}

/// Cached letter-spacing correction for one cell's char, if known.
fn correction_for(params: &FontParams, c: char, weight: u16) -> Option<f32> {
    if !correctable(c) {
        return None;
    }
    let key = (params.family.clone().unwrap_or_default(), weight, c);
    CORRECTION_CACHE.with(|cache| cache.borrow().get(&key).copied().flatten())
}

/// Walk the shaped layout looking for width-1 glyphs whose rounded advance
/// is not one cell (a narrow fallback glyph rounds to ZERO and shifts the
/// row left; an over-wide one rounds to two and shifts it right). Each
/// offender's natural advance is measured from the font that actually
/// shaped it and its correction cached. Returns whether anything new was
/// cached — i.e. whether the caller must rebuild the buffer.
fn detect_corrections(buffer: &Buffer, font_system: &mut FontSystem, params: &FontParams) -> bool {
    let cell_em = params.cell_w / params.font_size;
    // Collect offenders first: (char, weight, font_id, glyph_id).
    let mut offenders: Vec<(char, u16, fontdb::ID, u16)> = Vec::new();
    for run in buffer.layout_runs() {
        for g in run.glyphs {
            let Some(c) = run.text[g.start..g.end].chars().next() else {
                continue;
            };
            if !correctable(c) || (g.w / params.cell_w).round() as i64 == 1 {
                continue;
            }
            offenders.push((c, g.font_weight.0, g.font_id, g.glyph_id));
        }
    }
    let mut cached_new = false;
    for (c, weight, font_id, glyph_id) in offenders {
        let key = (params.family.clone().unwrap_or_default(), weight, c);
        let known = CORRECTION_CACHE.with(|cache| cache.borrow().contains_key(&key));
        if known {
            continue;
        }
        let ls = font_system
            .get_font(font_id, fontdb::Weight(weight))
            .and_then(|font| {
                let a_em = font
                    .as_swash()
                    .glyph_metrics(&[])
                    .scale(1.0)
                    .advance_width(glyph_id);
                cell_correction_em(a_em, cell_em)
            });
        cached_new |= ls.is_some();
        // Tradeoff: a `None` here (get_font failed, or the measured advance
        // rounds to one cell anyway) is cached permanently to prevent
        // detect/rebuild loops — that glyph stays as-is with no retry path.
        CORRECTION_CACHE.with(|cache| cache.borrow_mut().insert(key, ls));
    }
    cached_new
}

/// Per-column styling key, used to coalesce horizontally-adjacent cells that
/// share a style into one shaping span. `Default` = an empty cell (rendered as a
/// space in the buffer's default attrs).
#[derive(PartialEq)]
enum RunKey {
    Default,
    /// fg, bold, italic, letter-spacing correction (f32 bits — `None` for
    /// glyphs whose natural advance already snaps to their cell).
    Styled((u8, u8, u8), bool, bool, Option<u32>),
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
    params: &FontParams,
) {
    let fam = family_from(&params.family);
    let base = Weight(params.weight);
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
                Some(cell) => {
                    let w = if cell.bold {
                        Weight::BOLD.0
                    } else {
                        params.weight
                    };
                    let ls = correction_for(params, cell.c, w);
                    (
                        cell.c,
                        RunKey::Styled(cell.fg, cell.bold, cell.italic, ls.map(f32::to_bits)),
                    )
                }
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
                RunKey::Styled(fg, bold, italic, ls) => {
                    let mut a = Attrs::new()
                        .family(fam)
                        .color(Color::rgb(fg.0, fg.1, fg.2))
                        .weight(if *bold { Weight::BOLD } else { base });
                    if *italic {
                        a = a.style(Style::Italic);
                    }
                    if let Some(bits) = ls {
                        a = a.letter_spacing(f32::from_bits(*bits));
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
