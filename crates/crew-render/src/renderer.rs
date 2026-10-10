use std::sync::Arc;

use winit::window::Window;

use crate::cellgrid::CellGrid;
use crate::crtchain::CrtChain;
use crate::fadepass::FadePass;
use crate::gpu::Gpu;
use crate::scene::PaneScene;
use crate::solidcard::SolidCardPass;

/// Never let the window go so sheer that it is nothing at all — a page at 0
/// is a way to lose the app entirely (and the window server passes clicks on
/// fully clear pixels through to whatever is behind). The Opacity % slider
/// stops far above this (crew-app's own floor, 0.35). A CRT tube's window
/// asked for 0.12 for a day; its faceplate (crew-app's `tubesheer`) sits at
/// 0.84 now, so nothing asks for less than the slider's floor.
const MIN_WINDOW_OPACITY: f32 = 0.1;

/// Top-level renderer: owns `Gpu` + `CellGrid` and orchestrates the full frame.
pub struct Renderer {
    gpu: Gpu,
    cell_grid: CellGrid,
    /// Alpha the page background is cleared/drawn with. `1.0` is the opaque
    /// window; lower lets the desktop through (see [`Self::set_window_opacity`]).
    window_opacity: f32,
    /// Hands the focused card and crew's own chrome their opacity back while
    /// the window is sheer, so neither the surface being read nor the bar
    /// being typed into has the desktop behind it.
    solid_card: SolidCardPass,
    /// Rects that stay solid however sheer the window is, in physical px —
    /// see [`Self::set_solid_chrome`].
    solid_chrome: Vec<[f32; 4]>,
    // CRT post-process: when a style is set, the frame renders into the
    // chain's scene target then reprojects (bloom + composite); otherwise it
    // draws straight to the surface.
    crt: CrtChain,
    // Theme crossfade: the last presented frame, drawn back over the new
    // theme's frames at `theme_fade` strength while a switch settles.
    fade: FadePass,
    theme_fade: Option<f32>,
}

impl Renderer {
    pub fn new(window: Arc<Window>, font_size: f32) -> anyhow::Result<Self> {
        let gpu = Gpu::new(window)?;
        let cell_grid = CellGrid::new(gpu.device(), gpu.queue(), gpu.format, font_size);
        let solid_card = SolidCardPass::new(gpu.device(), gpu.format);
        let crt = CrtChain::new(
            gpu.device(),
            gpu.format,
            gpu.config.width,
            gpu.config.height,
        );
        let fade = FadePass::new(
            gpu.device(),
            gpu.format,
            gpu.config.width,
            gpu.config.height,
            gpu.surface_copy,
        );
        Ok(Self {
            gpu,
            cell_grid,
            window_opacity: 1.0,
            solid_card,
            solid_chrome: Vec::new(),
            crt,
            fade,
            theme_fade: None,
        })
    }

    /// Update the font size at runtime; recomputes cell metrics immediately.
    pub fn set_font_size(&mut self, font_size: f32) {
        self.cell_grid.set_font_size(font_size);
    }

    /// Set the cell height as a fraction of the font size — the user's
    /// `/leading`. Idempotent, so callers can set it beside the font size on
    /// every config adoption without re-warming the atlas.
    pub fn set_leading(&mut self, leading: f32) {
        self.cell_grid.set_leading(leading);
    }

    /// Switch the font family at runtime (`None`/empty → system monospace).
    pub fn set_font_family(&mut self, family: Option<String>) {
        self.cell_grid.set_font_family(family);
    }

    /// The family text is drawn in — what [`Self::set_font_family`] was given,
    /// unless that would not land on the cell grid (`None` = the embedded
    /// face).
    pub fn font_family(&self) -> Option<&str> {
        self.cell_grid.font_family()
    }

    /// Override the base text weight (CSS scale; `None` → theme default).
    pub fn set_font_weight(&mut self, weight: Option<u16>) {
        self.cell_grid.set_font_weight(weight);
    }

    /// Override the CoreText-style smoothing strength (0–255, 0 = off;
    /// `None` → built-in default).
    pub fn set_text_smoothing(&mut self, strength: Option<u8>) {
        self.cell_grid.set_text_smoothing(strength);
    }

    /// Override the coverage-curve amount (0–255, 0 = off; `None` follows
    /// [`crew_render::DEFAULT_TEXT_GAMMA`](crate::DEFAULT_TEXT_GAMMA)).
    pub fn set_text_gamma(&mut self, amount: Option<u8>) {
        self.cell_grid.set_text_gamma(amount);
    }

    /// Theme-switch crossfade: how strongly the LAST presented frame still
    /// covers the new theme's frame (1 → all old). Pass `None` (or ≤ 0) once
    /// settled. The caller owns the fade timeline; this is the per-frame value.
    pub fn set_theme_fade(&mut self, fade: Option<f32>) {
        self.theme_fade = fade.filter(|a| *a > 0.001);
    }

    pub fn set_glass(&mut self, level: crew_theme::GlassLevel) {
        self.cell_grid.set_glass(level);
    }

    /// Lean the glass rims' light toward a tilt in `-1..=1` per axis (the
    /// pointer's place in the window); `(0, 0)` is the resting light.
    pub fn set_glass_light(&mut self, tilt: (f32, f32)) {
        self.cell_grid.set_glass_light(tilt);
    }

    /// Set the window's opacity (1.0 = fully opaque). Below 1.0 the desktop
    /// shows through everything crew draws.
    pub fn set_window_opacity(&mut self, opacity: f32) {
        self.window_opacity = opacity.clamp(MIN_WINDOW_OPACITY, 1.0);
        self.cell_grid.set_window_opacity(self.window_opacity);
    }

    /// The rects a sheer window keeps solid whatever has focus — crew's own
    /// furniture, in physical px. The app sets them each frame because the
    /// layout is its to know; an empty list leaves only the focused card
    /// solid. Ignored entirely at full opacity.
    pub fn set_solid_chrome(&mut self, rects: Vec<[f32; 4]>) {
        self.solid_chrome = rects;
    }

    /// Set the CRT tube post-process style; `None` turns it off and the frame
    /// draws straight to the surface with no extra pass (the original path).
    pub fn set_crt(&mut self, style: Option<crew_theme::CrtStyle>) {
        self.crt.set_style(style);
    }

    /// Per-frame CRT animation: `time` seeds the flicker hash, `flicker` is its
    /// amplitude (0 = a static tube). The app lifts these only while streaming.
    pub fn set_crt_anim(&mut self, time: f32, flicker: f32) {
        self.crt.set_anim(time, flicker);
    }

    /// Sorted, de-duplicated names of all installed monospace font families.
    pub fn monospace_families(&mut self) -> Vec<String> {
        self.cell_grid.monospace_families()
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.gpu.resize(w, h);
        self.cell_grid.resize(w as f32, h as f32);
        // The off-screen CRT + bloom targets track the surface size.
        self.crt.resize(
            self.gpu.device(),
            self.gpu.config.width,
            self.gpu.config.height,
        );
        self.fade.resize(
            self.gpu.device(),
            self.gpu.config.width,
            self.gpu.config.height,
        );
    }

    /// Returns the monospace cell size `(width, height)` in pixels.
    pub fn cell_size(&self) -> (f32, f32) {
        self.cell_grid.cell_size()
    }

    /// Returns the current surface dimensions `(width, height)` in pixels.
    pub fn surface_size(&self) -> (u32, u32) {
        (self.gpu.config.width, self.gpu.config.height)
    }

    /// Upload a scene of panes, render, and present the frame — the heavy
    /// lifting lives in [`crate::frame::render`].
    pub fn frame(&mut self, panes: &[PaneScene]) {
        // Per frame: a theme switch moves it as surely as an opacity change.
        self.crt
            .set_premultiply(crate::crtchain::premultiplies(self.window_opacity));
        self.crt.set_sheer(self.window_opacity < 1.0);
        crate::frame::render(
            &self.gpu,
            &mut self.cell_grid,
            &self.crt,
            &mut self.fade,
            self.theme_fade,
            &mut self.solid_card,
            &self.solid_chrome,
            self.window_opacity,
            panes,
        );
    }
}
