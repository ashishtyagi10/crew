//! Helper to prepare one glyphon `TextRenderer` from a set of pane buffers.
//! Factored out so `CellGrid` can drive two renderers (base panes and overlay
//! popups) without duplicating the TextArea setup.
use glyphon::{Color, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport};

use crate::scene::PaneBuffer;
use crate::smoothing::presmooth;

/// Default text colour for glyphs that don't carry their own (Crew's accent).
const DEFAULT_TEXT: Color = Color::rgb(0, 255, 160);

/// Lay out `buffers` into clipped `TextArea`s and upload them into `renderer`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_renderer(
    renderer: &mut TextRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font_system: &mut glyphon::FontSystem,
    atlas: &mut TextAtlas,
    viewport: &Viewport,
    buffers: &[PaneBuffer],
    swash: &mut SwashCache,
) {
    // Seed the swash cache with smoothed glyph bitmaps before glyphon's
    // prepare walks the same glyphs — its cache hits take our images.
    presmooth(swash, font_system, buffers);

    let areas: Vec<TextArea<'_>> = buffers
        .iter()
        .map(|(buf, ox, oy, pw, ph)| TextArea {
            buffer: buf,
            left: *ox,
            top: *oy,
            scale: 1.0,
            bounds: TextBounds {
                left: *ox as i32,
                top: *oy as i32,
                right: (*ox + *pw) as i32,
                bottom: (*oy + *ph) as i32,
            },
            default_color: DEFAULT_TEXT,
            custom_glyphs: &[],
        })
        .collect();

    renderer
        .prepare(device, queue, font_system, atlas, viewport, areas, swash)
        .expect("glyphon prepare failed");
}
