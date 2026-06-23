use glyphon::{Cache, FontSystem, Resolution, SwashCache, TextAtlas, TextRenderer, Viewport};

use crate::celltext::{cell_metrics, monospace_families, FontParams};
use crate::gpu::Gpu;
use crate::quads::QuadLayer;
use crate::roundborder::RoundBorderLayer;
use crate::scene::{build_scene, PaneBuffer, PaneScene};
use crate::textprep::prepare_renderer;

/// Default terminal background colour (must match scene.rs).
pub(crate) const DEFAULT_BG: (u8, u8, u8) = (0, 0, 0);

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

/// Renders a scene of panes: per-cell bg quads, rounded borders, per-pane text.
pub struct CellGrid {
    pub(crate) font_system: FontSystem,
    swash: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    /// Second renderer for overlay popups, drawn after base panes so nothing
    /// behind them bleeds through.
    overlay_renderer: TextRenderer,
    /// One Buffer per pane, plus (origin_x, origin_y, pane_w, pane_h).
    pane_buffers: Vec<PaneBuffer>,
    overlay_buffers: Vec<PaneBuffer>,
    quad_layer: QuadLayer,
    overlay_quad_layer: QuadLayer,
    round_border_layer: RoundBorderLayer,
    pub(crate) cell_w: f32,
    pub(crate) cell_h: f32,
    font_size: f32,
    line_height: f32,
    font_family: Option<String>,
}

impl CellGrid {
    pub fn new(gpu: &Gpu, font_size: f32) -> Self {
        let mut font_system = FontSystem::new();
        let swash = SwashCache::new();
        let cache = Cache::new(&gpu.device);
        let viewport = Viewport::new(&gpu.device, &cache);
        let mut atlas = TextAtlas::new(&gpu.device, &gpu.queue, &cache, gpu.format);
        let mk_renderer = |atlas: &mut TextAtlas| {
            TextRenderer::new(atlas, &gpu.device, wgpu::MultisampleState::default(), None)
        };
        let renderer = mk_renderer(&mut atlas);
        let overlay_renderer = mk_renderer(&mut atlas);

        let font_family: Option<String> = None;
        let (cell_w, cell_h) = cell_metrics(&mut font_system, font_size, &font_family);
        let line_height = font_size * 1.25;
        let quad_layer = QuadLayer::new(&gpu.device, gpu.format);
        let overlay_quad_layer = QuadLayer::new(&gpu.device, gpu.format);
        let round_border_layer = RoundBorderLayer::new(&gpu.device, gpu.format);

        Self {
            font_system,
            swash,
            viewport,
            atlas,
            renderer,
            overlay_renderer,
            pane_buffers: Vec::new(),
            overlay_buffers: Vec::new(),
            quad_layer,
            overlay_quad_layer,
            round_border_layer,
            cell_w,
            cell_h,
            font_size,
            line_height,
            font_family,
        }
    }

    /// Update cell metrics when the font size changes at runtime.
    pub fn set_font_size(&mut self, font_size: f32) {
        let (cell_w, cell_h) = cell_metrics(&mut self.font_system, font_size, &self.font_family);
        self.font_size = font_size;
        self.line_height = font_size * 1.25;
        self.cell_w = cell_w;
        self.cell_h = cell_h;
    }

    /// Switch the font family at runtime (`None`/empty → system monospace) and
    /// recompute cell metrics for the new face.
    pub fn set_font_family(&mut self, family: Option<String>) {
        self.font_family = family.filter(|n| !n.is_empty());
        let (cell_w, cell_h) =
            cell_metrics(&mut self.font_system, self.font_size, &self.font_family);
        self.cell_w = cell_w;
        self.cell_h = cell_h;
    }

    /// Sorted, de-duplicated names of all installed monospace font families.
    pub fn monospace_families(&self) -> Vec<String> {
        monospace_families(&self.font_system)
    }

    /// Returns the monospace cell size `(width, height)` in pixels.
    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_w, self.cell_h)
    }

    /// Update the text buffer layout bounds on resize (no-op now; sizing per pane).
    pub fn resize(&mut self, _width: f32, _height: f32) {}

    /// Upload a scene of panes: backgrounds as quads, rounded borders, one Buffer per pane.
    pub fn set_scene(&mut self, gpu: &Gpu, panes: &[PaneScene]) {
        let params = FontParams {
            font_size: self.font_size,
            line_height: self.line_height,
            family: self.font_family.clone(),
        };
        let (cw, ch) = (self.cell_w, self.cell_h);
        let (quads, buffers, borders) =
            build_scene(panes, cw, ch, &mut self.font_system, &params, false);
        let (oquads, obuffers, _) =
            build_scene(panes, cw, ch, &mut self.font_system, &params, true);
        self.quad_layer.set_quads(&gpu.device, &quads);
        self.overlay_quad_layer.set_quads(&gpu.device, &oquads);
        self.round_border_layer.set_borders(&gpu.device, &borders);
        self.pane_buffers = buffers;
        self.overlay_buffers = obuffers;
    }

    /// Update viewports and prepare GPU uploads for all pane text areas.
    pub fn prepare(&mut self, gpu: &Gpu) {
        let w = gpu.config.width as f32;
        let h = gpu.config.height as f32;
        self.quad_layer.set_viewport(&gpu.queue, w, h);
        self.overlay_quad_layer.set_viewport(&gpu.queue, w, h);
        self.round_border_layer.set_viewport(&gpu.queue, w, h);
        self.viewport.update(
            &gpu.queue,
            Resolution {
                width: gpu.config.width,
                height: gpu.config.height,
            },
        );

        prepare_renderer(
            &mut self.renderer,
            gpu,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            &self.pane_buffers,
            &mut self.swash,
        );
        prepare_renderer(
            &mut self.overlay_renderer,
            gpu,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            &self.overlay_buffers,
            &mut self.swash,
        );
    }

    /// Draw base panes (backgrounds → borders → text), then overlay popups
    /// (backgrounds → text) on top, so overlays are fully opaque — no pane text
    /// behind them can bleed through.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.quad_layer.draw(pass);
        self.round_border_layer.draw(pass);
        self.renderer
            .render(&self.atlas, &self.viewport, pass)
            .expect("glyphon render failed");
        self.overlay_quad_layer.draw(pass);
        self.overlay_renderer
            .render(&self.atlas, &self.viewport, pass)
            .expect("glyphon overlay render failed");
    }
}
