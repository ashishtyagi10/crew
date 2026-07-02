use glyphon::{Cache, FontSystem, Resolution, SwashCache, TextAtlas, TextRenderer, Viewport};

use crate::celltext::{cell_metrics, monospace_families, FontParams};
use crate::quads::QuadLayer;
use crate::roundborder::RoundBorderLayer;
use crate::scene::{build_both, PaneScene};
use crate::scenecache::SceneSlots;
use crate::textprep::prepare_renderer;

/// The active theme's default background (the page colour). Cells at this bg
/// skip their bg quad and let the cleared page show through.
pub(crate) fn default_bg() -> (u8, u8, u8) {
    crew_theme::theme().page_bg
}

/// A single terminal cell to be rendered.
#[derive(Hash)]
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
    /// Retained shaped buffers + signatures per pass, so unchanged panes
    /// reuse last frame's shaping (see [`crate::scenecache`]).
    base: SceneSlots,
    overlay: SceneSlots,
    quad_layer: QuadLayer,
    overlay_quad_layer: QuadLayer,
    round_border_layer: RoundBorderLayer,
    pub(crate) cell_w: f32,
    pub(crate) cell_h: f32,
    font_size: f32,
    line_height: f32,
    font_family: Option<String>,
    /// Whether the render target is sRGB (colours must be fed linear).
    srgb: bool,
}

impl CellGrid {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        font_size: f32,
    ) -> Self {
        let font_system = FontSystem::new();
        let swash = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let mk_renderer = |atlas: &mut TextAtlas| {
            TextRenderer::new(atlas, device, wgpu::MultisampleState::default(), None)
        };
        let renderer = mk_renderer(&mut atlas);
        let overlay_renderer = mk_renderer(&mut atlas);

        let font_family: Option<String> = None;
        let (cell_w, cell_h) = cell_metrics(font_size);
        let line_height = font_size * 1.25;
        let quad_layer = QuadLayer::new(device, format);
        let overlay_quad_layer = QuadLayer::new(device, format);
        let round_border_layer = RoundBorderLayer::new(device, format);

        Self {
            font_system,
            swash,
            viewport,
            atlas,
            renderer,
            overlay_renderer,
            base: SceneSlots::default(),
            overlay: SceneSlots::default(),
            quad_layer,
            overlay_quad_layer,
            round_border_layer,
            cell_w,
            cell_h,
            font_size,
            line_height,
            font_family,
            srgb: format.is_srgb(),
        }
    }

    /// Update cell metrics when the font size changes at runtime.
    pub fn set_font_size(&mut self, font_size: f32) {
        let (cell_w, cell_h) = cell_metrics(font_size);
        self.font_size = font_size;
        self.line_height = font_size * 1.25;
        self.cell_w = cell_w;
        self.cell_h = cell_h;
    }

    /// Switch the font family at runtime (`None`/empty → system monospace).
    /// The cell box is fixed per font size — glyphs snap to it at layout time —
    /// so no metrics change and the grid never moves.
    pub fn set_font_family(&mut self, family: Option<String>) {
        self.font_family = family.filter(|n| !n.is_empty());
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
    pub fn set_scene(&mut self, device: &wgpu::Device, panes: &[PaneScene]) {
        let params = FontParams {
            font_size: self.font_size,
            line_height: self.line_height,
            cell_w: self.cell_w,
            family: self.font_family.clone(),
        };
        let (cw, ch) = (self.cell_w, self.cell_h);
        let ((quads, buffers, sigs, borders), (oquads, obuffers, osigs, _)) = build_both(
            panes,
            cw,
            ch,
            &mut self.font_system,
            &params,
            self.srgb,
            self.base.take_prev(),
            self.overlay.take_prev(),
        );
        self.quad_layer.set_quads(device, &quads);
        self.overlay_quad_layer.set_quads(device, &oquads);
        self.round_border_layer.set_borders(device, &borders);
        self.base.set(sigs, buffers);
        self.overlay.set(osigs, obuffers);
    }

    /// Update viewports and prepare GPU uploads for all pane text areas.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        let w = width as f32;
        let h = height as f32;
        self.quad_layer.set_viewport(queue, w, h);
        self.overlay_quad_layer.set_viewport(queue, w, h);
        self.round_border_layer.set_viewport(queue, w, h);
        self.viewport.update(queue, Resolution { width, height });

        prepare_renderer(
            &mut self.renderer,
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            self.base.bufs(),
            &mut self.swash,
        );
        prepare_renderer(
            &mut self.overlay_renderer,
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            self.overlay.bufs(),
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
