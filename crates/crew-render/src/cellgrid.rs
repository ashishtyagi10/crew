use glyphon::{
    Cache, Color, FontSystem, Resolution, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};

use crate::celltext::{cell_metrics, FontParams};
use crate::gpu::Gpu;
use crate::quads::QuadLayer;
use crate::scene::{build_scene, PaneBuffer, PaneScene};

/// Default terminal background colour (must match scene.rs).
pub(crate) const DEFAULT_BG: (u8, u8, u8) = (8, 8, 16);

/// A single terminal cell to be rendered.
pub struct CellView {
    pub col: u16,
    pub row: u16,
    pub c: char,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
    pub bold: bool,
    pub italic: bool,
}

/// Renders a scene of panes: per-cell bg quads, pane borders, per-pane text.
pub struct CellGrid {
    pub(crate) font_system: FontSystem,
    swash: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    /// One Buffer per pane, plus (origin_x, origin_y, pane_w, pane_h).
    pane_buffers: Vec<PaneBuffer>,
    quad_layer: QuadLayer,
    pub(crate) cell_w: f32,
    pub(crate) cell_h: f32,
    font_size: f32,
    line_height: f32,
}

impl CellGrid {
    pub fn new(gpu: &Gpu, font_size: f32) -> Self {
        let mut font_system = FontSystem::new();
        let swash = SwashCache::new();
        let cache = Cache::new(&gpu.device);
        let viewport = Viewport::new(&gpu.device, &cache);
        let mut atlas = TextAtlas::new(&gpu.device, &gpu.queue, &cache, gpu.format);
        let renderer = TextRenderer::new(
            &mut atlas,
            &gpu.device,
            wgpu::MultisampleState::default(),
            None,
        );

        let (cell_w, cell_h) = cell_metrics(&mut font_system, font_size);
        let line_height = font_size * 1.25;
        let quad_layer = QuadLayer::new(&gpu.device, gpu.format);

        Self {
            font_system,
            swash,
            viewport,
            atlas,
            renderer,
            pane_buffers: Vec::new(),
            quad_layer,
            cell_w,
            cell_h,
            font_size,
            line_height,
        }
    }

    /// Update cell metrics when the font size changes at runtime.
    pub fn set_font_size(&mut self, font_size: f32) {
        let (cell_w, cell_h) = cell_metrics(&mut self.font_system, font_size);
        self.font_size = font_size;
        self.line_height = font_size * 1.25;
        self.cell_w = cell_w;
        self.cell_h = cell_h;
    }

    /// Returns the monospace cell size `(width, height)` in pixels.
    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_w, self.cell_h)
    }

    /// Update the text buffer layout bounds on resize (no-op now; sizing per pane).
    pub fn resize(&mut self, _width: f32, _height: f32) {}

    /// Upload a scene of panes: backgrounds + borders as quads, one Buffer per pane.
    pub fn set_scene(&mut self, gpu: &Gpu, panes: &[PaneScene]) {
        let params = FontParams {
            font_size: self.font_size,
            line_height: self.line_height,
        };
        let (quads, buffers) = build_scene(
            panes,
            self.cell_w,
            self.cell_h,
            &mut self.font_system,
            &params,
        );
        self.quad_layer.set_quads(&gpu.device, &quads);
        self.pane_buffers = buffers;
    }

    /// Update viewports and prepare GPU uploads for all pane text areas.
    pub fn prepare(&mut self, gpu: &Gpu) {
        self.quad_layer.set_viewport(
            &gpu.queue,
            gpu.config.width as f32,
            gpu.config.height as f32,
        );
        self.viewport.update(
            &gpu.queue,
            Resolution {
                width: gpu.config.width,
                height: gpu.config.height,
            },
        );

        let areas: Vec<TextArea<'_>> = self
            .pane_buffers
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
                default_color: Color::rgb(0, 255, 160),
                custom_glyphs: &[],
            })
            .collect();

        self.renderer
            .prepare(
                &gpu.device,
                &gpu.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                areas,
                &mut self.swash,
            )
            .expect("glyphon prepare failed");
    }

    /// Draw backgrounds + borders then all pane text into the active render pass.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.quad_layer.draw(pass);
        self.renderer
            .render(&self.atlas, &self.viewport, pass)
            .expect("glyphon render failed");
    }
}
