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

    // Each pane's snapped buffer, and the runs of joining script drawn whole
    // over it, clipped to the pane.
    let areas: Vec<TextArea<'_>> = crate::scriptrun::areas(buffers)
        .map(|(buf, ox, oy, [l, t, r, b])| TextArea {
            buffer: buf,
            left: ox,
            top: oy,
            scale: 1.0,
            bounds: TextBounds {
                left: l as i32,
                top: t as i32,
                right: r as i32,
                bottom: b as i32,
            },
            default_color: DEFAULT_TEXT,
            custom_glyphs: &[],
        })
        .collect();

    renderer
        .prepare(device, queue, font_system, atlas, viewport, areas, swash)
        .expect("glyphon prepare failed");
}
