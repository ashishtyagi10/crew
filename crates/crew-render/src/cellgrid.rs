use glyphon::{
    Cache, ColorMode, FontSystem, Resolution, SwashCache, TextAtlas, TextRenderer, Viewport,
};

use crate::celltext::{base_weight, cell_metrics, FontParams, CELL_H_RATIO};
use crate::fontlist::monospace_families;
use crate::glass::GlassLayer;
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

/// Glyph blending mode for a target of the given sRGB-ness. Non-sRGB targets
/// get `Web`: sRGB text colours pass through unconverted and the fixed-function
/// blend operates on gamma-encoded values — the browser/CoreText look the
/// smoothness work targets. If a platform only offers sRGB surfaces, `Accurate`
/// keeps colours correct (Web mode on an sRGB target would double-encode).
pub(crate) fn atlas_color_mode(srgb: bool) -> ColorMode {
    if srgb {
        ColorMode::Accurate
    } else {
        ColorMode::Web
    }
}

/// A single terminal cell to be rendered.
///
/// `Default` exists so every construction site can end in
/// `..Default::default()` and keep compiling when a new cell attribute lands —
/// there are eighty of them. Clippy's `needless_update` guards the other
/// direction: a literal that does name every field is told to drop the update.
#[derive(Hash, Default)]
pub struct CellView {
    pub col: u16,
    pub row: u16,
    pub c: char,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
    pub bold: bool,
    pub italic: bool,
    /// Underline family, strikethrough and SGR 58's colour. Drawn as quads,
    /// not glyphs — see [`crate::deco`].
    pub deco: crew_theme::deco::Deco,
    /// The cursor, when this is the cell it sits on.
    pub cursor: crew_theme::deco::CursorMark,
}

/// Renders a scene of panes: per-cell bg quads, rounded borders, per-pane text.
pub struct CellGrid {
    pub(crate) font_system: FontSystem,
    pub(crate) swash: SwashCache,
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
    /// Theme-switch veil: one full-window quad drawn over everything, fading
    /// out as the new theme develops. `None` (the resting state) draws nothing.
    round_border_layer: RoundBorderLayer,
    /// Frosted sheets drawn beneath everything else in the base pass.
    glass_layer: GlassLayer,
    /// How strong the glass is; `Off` builds no cards at all.
    glass_level: crew_theme::GlassLevel,
    pub(crate) cell_w: f32,
    pub(crate) cell_h: f32,
    font_size: f32,
    line_height: f32,
    /// Cell height as a fraction of the font size — the user's `/leading`.
    /// Held here rather than read from a global so `crew-render` stays a
    /// library with no opinion about where settings live.
    leading: f32,
    font_family: Option<String>,
    /// User base-weight override (CSS scale). `None` follows the theme default
    /// ([`base_weight`]); `Some(w)` renders all non-bold text at `w` so the
    /// user can make the body heavier/lighter. Bold cells still shape BOLD.
    weight_override: Option<u16>,
    /// CoreText-style smoothing strength override (0–255, 0 = off). `None`
    /// follows [`crate::smoothing::DEFAULT_SMOOTH`].
    smooth_override: Option<u8>,
    /// Text-gamma amount override (0–255, 0 = off). `None` follows
    /// [`crate::textgamma::DEFAULT_TEXT_GAMMA`].
    gamma_override: Option<u8>,
    /// Whether the render target is sRGB (colours must be fed linear).
    srgb: bool,
    /// Arms the glyph-atlas prewarm: set at construction and by every
    /// font-affecting setter, consumed by the next [`Self::prepare`].
    /// `pub(crate)` for the prewarm tests only.
    pub(crate) needs_prewarm: bool,
}

impl CellGrid {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        font_size: f32,
    ) -> Self {
        let font_system = crate::embedfont::font_system();
        let swash = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::with_color_mode(
            device,
            queue,
            &cache,
            format,
            atlas_color_mode(format.is_srgb()),
        );
        let mk_renderer = |atlas: &mut TextAtlas| {
            TextRenderer::new(atlas, device, wgpu::MultisampleState::default(), None)
        };
        let renderer = mk_renderer(&mut atlas);
        let overlay_renderer = mk_renderer(&mut atlas);

        let font_family: Option<String> = None;
        let leading = CELL_H_RATIO;
        let (cell_w, cell_h) = cell_metrics(font_size, leading);
        // Text rows must land exactly on the (rounded) cell grid, so the
        // buffer line height IS the cell height — never derived separately.
        let line_height = cell_h;
        let quad_layer = QuadLayer::new(device, format);
        let overlay_quad_layer = QuadLayer::new(device, format);
        let round_border_layer = RoundBorderLayer::new(device, format);
        let glass_layer = GlassLayer::new(device, format);

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
            glass_layer,
            glass_level: crew_theme::GlassLevel::Medium,
            cell_w,
            cell_h,
            font_size,
            leading,
            line_height,
            font_family,
            weight_override: None,
            smooth_override: None,
            gamma_override: None,
            srgb: format.is_srgb(),
            needs_prewarm: true,
        }
    }

    /// The `FontParams` this frame shapes with — one source for real scenes
    /// AND the prewarm buffer, so their cache keys agree by construction.
    pub(crate) fn font_params(&self) -> FontParams {
        FontParams {
            font_size: self.font_size,
            line_height: self.line_height,
            cell_w: self.cell_w,
            family: self.font_family.clone(),
            // A user weight override wins; otherwise the theme default (Medium
            // for crisp ink on a bright page). Per-frame theme read, same
            // pattern as page_bg in renderer.rs.
            weight: self
                .weight_override
                .unwrap_or_else(|| base_weight(crew_theme::theme().dark)),
            smooth: self
                .smooth_override
                .unwrap_or(crate::smoothing::DEFAULT_SMOOTH),
            gamma: self
                .gamma_override
                .unwrap_or(crate::textgamma::DEFAULT_TEXT_GAMMA),
            // Per-frame theme read, same as the weight above: the polarity
            // the coverage curve bends toward IS the page's.
            dark: crew_theme::theme().dark,
            body: (crew_theme::theme().ink, crew_theme::theme().page_bg),
        }
    }

    /// Update cell metrics when the font size changes at runtime.
    pub fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
        self.remeasure();
    }

    /// Update cell metrics when the leading changes at runtime. A no-op when
    /// the ratio has not moved: every caller sets this beside the font size
    /// on every config adoption, and re-measuring would throw away the glyph
    /// cache and re-warm the atlas for nothing.
    pub fn set_leading(&mut self, leading: f32) {
        if (leading - self.leading).abs() < f32::EPSILON {
            return;
        }
        self.leading = leading;
        self.remeasure();
    }

    /// Recompute the cell box and drop everything keyed to the old one.
    fn remeasure(&mut self) {
        let (cell_w, cell_h) = cell_metrics(self.font_size, self.leading);
        self.line_height = cell_h;
        self.cell_w = cell_w;
        self.cell_h = cell_h;
        self.swash.image_cache.clear();
        self.needs_prewarm = true;
    }

    /// Switch the font family at runtime (`None`/empty → system monospace).
    /// The cell box is fixed per font size — glyphs snap to it at layout time —
    /// so no metrics change and the grid never moves.
    /// Override the base text weight (CSS scale, e.g. 500 Medium, 600 SemiBold,
    /// 700 Bold). `None` follows the theme default. Applied next frame.
    pub fn set_font_weight(&mut self, weight: Option<u16>) {
        self.weight_override = weight;
        self.swash.image_cache.clear();
        self.needs_prewarm = true;
    }

    pub fn set_font_family(&mut self, family: Option<String>) {
        self.font_family = family.filter(|n| !n.is_empty());
        // The swash image cache retains every rasterized glyph (the presmooth
        // pass reads and seeds it); font changes re-key everything, so drop
        // the stale rasters rather than carrying them for the session.
        self.swash.image_cache.clear();
        self.needs_prewarm = true;
    }

    /// Override the CoreText-style smoothing strength (0–255, 0 = off).
    /// `None` follows the default. Applied next frame: the strength lives in
    /// every glyph's cache key, so changed panes re-shape and re-rasterize
    /// while stale atlas entries become evictable at the next `prepare`'s
    /// `trim` and age out under LRU pressure.
    pub fn set_text_smoothing(&mut self, strength: Option<u8>) {
        self.smooth_override = strength;
        self.swash.image_cache.clear();
        self.needs_prewarm = true;
    }

    /// Override the coverage-curve amount (0–255, 0 = off). Like the
    /// smoothing strength this re-keys every glyph, so the cached bitmaps go
    /// with it and the atlas re-warms.
    pub fn set_text_gamma(&mut self, amount: Option<u8>) {
        self.gamma_override = amount;
        self.swash.image_cache.clear();
        self.needs_prewarm = true;
    }

    /// Set the frosted-glass strength. Applied next frame.
    pub fn set_glass(&mut self, level: crew_theme::GlassLevel) {
        self.glass_level = level;
    }

    /// Sorted, de-duplicated names of all installed monospace font families
    /// (verified fixed-pitch — `&mut` because verification loads the faces).
    ///
    /// Screened twice, and the second screen is the one that counts: a family
    /// must also survive [`crate::fontverify::snaps_to_cells`], which shapes a
    /// probe through *this grid's* real size and weight. `fontlist` measures
    /// the face it found by name; this measures the face that shaping actually
    /// selects. A Windows box shipped a proportional render with the first
    /// check passing, because those were not the same font.
    pub fn monospace_families(&mut self) -> Vec<String> {
        let params = self.font_params();
        let (font_size, cell_w, weight) = (params.font_size, params.cell_w, params.weight);
        let mut fams = monospace_families(&mut self.font_system);
        fams.retain(|f| {
            crate::fontverify::snaps_to_cells(&mut self.font_system, f, font_size, cell_w, weight)
        });
        fams
    }

    /// Returns the monospace cell size `(width, height)` in pixels.
    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_w, self.cell_h)
    }

    /// Update the text buffer layout bounds on resize (no-op now; sizing per pane).
    pub fn resize(&mut self, _width: f32, _height: f32) {}

    /// Upload a scene of panes: backgrounds as quads, rounded borders, one Buffer per pane.
    pub fn set_scene(&mut self, device: &wgpu::Device, panes: &[PaneScene]) {
        let params = self.font_params();
        let (cw, ch) = (self.cell_w, self.cell_h);
        let ((quads, buffers, sigs, borders, cards), (oquads, obuffers, osigs, _, _)) = build_both(
            panes,
            cw,
            ch,
            &mut self.font_system,
            &params,
            self.srgb,
            // Theme-derived, per-frame: `/theme` and `/glass` both land here.
            crew_theme::glass_style().scaled(self.glass_level),
            self.base.take_prev(),
            self.overlay.take_prev(),
        );
        self.quad_layer.set_quads(device, &quads);
        self.overlay_quad_layer.set_quads(device, &oquads);
        self.round_border_layer.set_borders(device, &borders);
        self.glass_layer.set_cards(device, &cards);
        self.base.set(sigs, buffers);
        self.overlay.set(osigs, obuffers);
    }

    /// Update viewports and prepare GPU uploads for all pane text areas.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        // Open the frame's in-use window: `trim` clears the atlas's
        // `glyphs_in_use` so only glyphs this frame actually touches (the
        // prewarm and prepare passes below re-insert them) are protected from
        // LRU eviction. Without it the set is monotone, eviction is dead code,
        // and every font/smoothing change pins another full working set until
        // the atlas hits AtlasFull.
        self.atlas.trim();
        // One-shot atlas prewarm: at startup — and after any font-affecting
        // change, coalescing the whole setter burst into one pass — the
        // working set rasterizes here, through the same presmooth-seeded
        // path as the real panes below, so later frames find their glyphs
        // already packed and no grow churn lands mid-interaction.
        if self.needs_prewarm {
            self.needs_prewarm = false;
            let params = self.font_params();
            crate::prewarm::prewarm(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                &mut self.swash,
                &params,
            );
        }
        let w = width as f32;
        let h = height as f32;
        self.quad_layer.set_viewport(queue, w, h);
        self.overlay_quad_layer.set_viewport(queue, w, h);
        self.round_border_layer.set_viewport(queue, w, h);
        self.glass_layer.set_viewport(queue, w, h);
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

    /// Draw base panes (glass → backgrounds → borders → text), then overlay
    /// popups (backgrounds → text) on top, so overlays are fully opaque — no
    /// pane text behind them can bleed through.
    ///
    /// Glass goes first, over the paper background but under everything the
    /// pane itself draws: the sheet must tint the page it sits on, never the
    /// text or selection highlights sitting on the sheet.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.glass_layer.draw(pass);
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

#[cfg(test)]
mod tests {
    use glyphon::ColorMode;

    use super::atlas_color_mode;

    #[test]
    fn atlas_color_mode_matches_the_target_kind() {
        // Non-sRGB target → Web (gamma-space blending, sRGB values pass through).
        // sRGB-only platform → Accurate (values linearized; never wash out).
        assert_eq!(atlas_color_mode(false), ColorMode::Web);
        assert_eq!(atlas_color_mode(true), ColorMode::Accurate);
    }
}
