//! Rich-text buffer builder for CellGrid.
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, Style, Weight, Wrap};

use crate::cellgrid::CellView;

/// Font metrics shared across all pane buffers.
pub(crate) struct FontParams {
    pub font_size: f32,
    pub line_height: f32,
}

/// Shape "M" and return its advance as the monospace cell width.
pub(crate) fn probe_cell_width(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    font_size: f32,
) -> f32 {
    let attrs = Attrs::new().family(Family::Monospace);
    buffer.set_text(font_system, "M", &attrs, Shaping::Advanced, None);
    for run in buffer.layout_runs() {
        if let Some(g) = run.glyphs.first() {
            return g.w;
        }
    }
    font_size * 0.6
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

    fill_rich_text(&mut buffer, font_system, cells, cols, rows);
    buffer
}

/// Fill an existing `Buffer` with rich-text spans for `cells` laid out in cols×rows.
pub(crate) fn fill_rich_text(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    cells: &[CellView],
    cols: usize,
    rows: usize,
) {
    // Bucket cells into a 2-D grid (row × col).
    let mut grid: Vec<Vec<Option<&CellView>>> = vec![vec![None; cols]; rows];
    for cell in cells {
        let r = cell.row as usize;
        let c = cell.col as usize;
        if r < rows && c < cols {
            grid[r][c] = Some(cell);
        }
    }

    let default_attrs = Attrs::new().family(Family::Monospace);

    // Collect span strings + attrs; keep strings alive so we can borrow them.
    let mut span_strings: Vec<String> = Vec::new();
    let mut span_attrs: Vec<Attrs<'static>> = Vec::new();

    for (row_i, row) in grid.iter().enumerate() {
        for cell_opt in row.iter() {
            let (ch, attrs) = match cell_opt {
                Some(cell) => {
                    let mut a = Attrs::new()
                        .family(Family::Monospace)
                        .color(Color::rgb(cell.fg.0, cell.fg.1, cell.fg.2));
                    if cell.bold {
                        a = a.weight(Weight::BOLD);
                    }
                    if cell.italic {
                        a = a.style(Style::Italic);
                    }
                    (cell.c.to_string(), a)
                }
                None => (" ".to_string(), default_attrs.clone()),
            };
            span_strings.push(ch);
            span_attrs.push(attrs);
        }
        if row_i + 1 < rows {
            span_strings.push("\n".to_string());
            span_attrs.push(default_attrs.clone());
        }
    }

    let spans: Vec<(&str, Attrs<'_>)> = span_strings
        .iter()
        .zip(span_attrs.iter())
        .map(|(s, a)| (s.as_str(), a.clone()))
        .collect();

    buffer.set_rich_text(font_system, spans, &default_attrs, Shaping::Advanced, None);
}

/// Compute monospace cell dimensions for the given font size without a GPU.
/// Returns `(cell_w, cell_h)` where `cell_h = font_size * 1.25`.
pub(crate) fn cell_metrics(fs: &mut FontSystem, font_size: f32) -> (f32, f32) {
    let cell_h = font_size * 1.25;
    let mut probe_buf = Buffer::new(fs, Metrics::new(font_size, cell_h));
    probe_buf.set_wrap(fs, Wrap::None);
    probe_buf.set_size(fs, Some(4096.0), Some(4096.0));
    let cell_w = probe_cell_width(&mut probe_buf, fs, font_size);
    (cell_w, cell_h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_metrics_larger_font_gives_larger_dimensions() {
        let mut fs = FontSystem::new();
        let small = cell_metrics(&mut fs, 12.0);
        let large = cell_metrics(&mut fs, 24.0);
        assert!(
            large.0 > small.0,
            "cell_w(24) {:.2} should be > cell_w(12) {:.2}",
            large.0,
            small.0
        );
        assert!(
            large.1 > small.1,
            "cell_h(24) {:.2} should be > cell_h(12) {:.2}",
            large.1,
            small.1
        );
        assert_eq!(large.1, 24.0 * 1.25, "cell_h(24) must equal 24.0 * 1.25");
    }
}
