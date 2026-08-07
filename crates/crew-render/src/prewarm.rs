//! Glyph-atlas prewarm: rasterize the working set once, up front.
//!
//! First frames used to pay for every glyph on screen — swash
//! rasterization, atlas packing and, at Retina sizes, atlas grows that
//! re-upload everything already packed. This module shapes one off-screen
//! buffer holding printable ASCII (base weight and bold) plus the
//! box-drawing/block/braille chars crew's chrome draws, and runs it through
//! the SAME [`prepare_renderer`] path real frames use: `presmooth` seeds
//! the stem-darkened bitmaps first, so the atlas is warmed with exactly the
//! images a real frame would upload — never a raw re-raster that would
//! poison the cache with unsmoothed glyphs.
//!
//! [`CellGrid::prepare`](crate::cellgrid::CellGrid::prepare) runs it once
//! at startup and re-runs it whenever the font family/size/weight/smoothing
//! changes (those setters clear the swash cache, so the seeds are stale).
use glyphon::{FontSystem, SwashCache, TextAtlas, TextRenderer, Viewport};

use crate::cellgrid::CellView;
use crate::celltext::{build_pane_buffer, FontParams};
use crate::scene::PaneBuffer;
use crate::textprep::prepare_renderer;

/// Non-ASCII glyphs crew's chrome actually draws (collected from the source
/// literals): pane borders and reply-chain connectors, block/bar meters,
/// braille spinner frames, markers, arrows, and the typographic punctuation
/// the chat pane leans on.
const CHROME: &str = "─│┌┐└┘├┤┬┴┼╭╮╯╰┃┆═║╔╗╚╝\
                      ▁▂▃▄▅▆▇█▉▊▋▌▍▎▏▐░▒▓\
                      ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏\
                      ▶▸○●⏺✓✗⚑☐•·…–—‘’“”›❯←↑→↓↔↵⇡⌘×≤≥";

/// Grid width of the prewarm buffer — only affects its shape, not its
/// contents; wide enough to keep the buffer a handful of rows tall.
const COLS: usize = 48;

/// Every `(char, bold)` the prewarm rasterizes: printable ASCII at both the
/// base weight and bold (legends and selections render bold), chrome
/// symbols at base weight only (borders never embolden).
pub(crate) fn working_set() -> Vec<(char, bool)> {
    let ascii = ' '..='~';
    ascii
        .clone()
        .map(|c| (c, false))
        .chain(CHROME.chars().map(|c| (c, false)))
        .chain(ascii.map(|c| (c, true)))
        .collect()
}

/// Shape the working set into one off-screen pane buffer with the exact
/// `FontParams` real panes use — same shaping, same cache-key flags, so
/// every seeded key is the key a real frame will look up. The origin is
/// (0, 0): pane rects snap to device pixels (v0.13.6) and cell metrics are
/// whole pixels, so all glyphs land in the same zero subpixel bin.
pub(crate) fn build_buffer(font_system: &mut FontSystem, params: &FontParams) -> PaneBuffer {
    let set = working_set();
    let cells: Vec<CellView> = set
        .iter()
        .enumerate()
        .map(|(i, &(c, bold))| CellView {
            col: (i % COLS) as u16,
            row: (i / COLS) as u16,
            c,
            fg: (255, 255, 255),
            bg: (0, 0, 0),
            bold,
            italic: false,
        })
        .collect();
    let rows = set.len().div_ceil(COLS);
    let (w, h) = (
        COLS as f32 * params.cell_w,
        rows as f32 * params.line_height,
    );
    let buf = build_pane_buffer(font_system, &cells, COLS, rows, w, h, params);
    (buf, 0.0, 0.0, w, h)
}

/// Rasterize and upload the working set through the real render path. A
/// scratch renderer keeps the prewarm vertices away from the live ones; the
/// seeded swash cache and the packed atlas are the outputs that persist.
/// ~280 glyphs — a one-off few milliseconds on the main thread.
pub(crate) fn prewarm(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font_system: &mut FontSystem,
    atlas: &mut TextAtlas,
    viewport: &Viewport,
    swash: &mut SwashCache,
    params: &FontParams,
) {
    let buffers = [build_buffer(font_system, params)];
    let mut scratch = TextRenderer::new(atlas, device, wgpu::MultisampleState::default(), None);
    prepare_renderer(
        &mut scratch,
        device,
        queue,
        font_system,
        atlas,
        viewport,
        &buffers,
        swash,
    );
}

#[cfg(test)]
#[path = "prewarm_tests.rs"]
mod tests;
