use std::sync::Arc;

use winit::window::Window;

use crate::cellgrid::CellGrid;
use crate::crtchain::CrtChain;
use crate::gpu::Gpu;
use crate::paperbg::PaperBgPass;
use crate::scene::PaneScene;

/// Never let the window go so sheer that crew becomes unreadable (or, worse,
/// unclickable-looking) — a translucency slider that can reach 0 is a way to
/// lose the app entirely.
const MIN_WINDOW_OPACITY: f32 = 0.35;

/// Top-level renderer: owns `Gpu` + `CellGrid` and orchestrates the full frame.
pub struct Renderer {
    gpu: Gpu,
    cell_grid: CellGrid,
    paper_bg: PaperBgPass,
    paper_texture: bool,
    paper_grain: f32,
    /// Alpha the page background is cleared/drawn with. `1.0` is the opaque
    /// window; lower lets the desktop through (see [`Self::set_window_opacity`]).
    window_opacity: f32,
    // CRT post-process: when a style is set, the frame renders into the
    // chain's scene target then reprojects (bloom + composite); otherwise it
    // draws straight to the surface.
    crt: CrtChain,
}

impl Renderer {
    pub fn new(window: Arc<Window>, font_size: f32) -> anyhow::Result<Self> {
        let gpu = Gpu::new(window)?;
        let cell_grid = CellGrid::new(&gpu.device, &gpu.queue, gpu.format, font_size);
        let paper_bg = PaperBgPass::new(&gpu.device, gpu.format);
        let crt = CrtChain::new(&gpu.device, gpu.format, gpu.config.width, gpu.config.height);
        Ok(Self {
            gpu,
            cell_grid,
            paper_bg,
            paper_texture: true,
            // Matches config's default_paper_grain; the app calls set_paper_grain
            // right after construction, so this is just a sane standalone default.
            paper_grain: 1.3,
            window_opacity: 1.0,
            crt,
        })
    }

    /// Update the font size at runtime; recomputes cell metrics immediately.
    pub fn set_font_size(&mut self, font_size: f32) {
        self.cell_grid.set_font_size(font_size);
    }

    /// Switch the font family at runtime (`None`/empty → system monospace).
    pub fn set_font_family(&mut self, family: Option<String>) {
        self.cell_grid.set_font_family(family);
    }

    /// Override the base text weight (CSS scale; `None` → theme default).
    pub fn set_font_weight(&mut self, weight: Option<u16>) {
        self.cell_grid.set_font_weight(weight);
    }

    /// Enable or disable the paper grain + vignette background pass.
    pub fn set_paper_texture(&mut self, enabled: bool) {
        self.paper_texture = enabled;
    }

    /// Set the frosted-glass strength for pane cards. The per-theme look is
    /// derived from the active theme, so this is only the intensity knob.
    pub fn set_glass(&mut self, level: crew_theme::GlassLevel) {
        self.cell_grid.set_glass(level);
    }

    /// Set the window's opacity (1.0 = fully opaque). Below 1.0 the desktop
    /// shows through everything crew draws.
    pub fn set_window_opacity(&mut self, opacity: f32) {
        self.window_opacity = opacity.clamp(MIN_WINDOW_OPACITY, 1.0);
    }

    /// Set the grain amplitude multiplier (0.0 = no grain, 1.0 = default ~±3%, 2.0 = double).
    /// This stores the USER knob only; the active theme's `grain`
    /// multiplies it at frame time in `frame()`, so light themes render
    /// noticeably grainier newsprint without changing what's stored here.
    pub fn set_paper_grain(&mut self, grain: f32) {
        self.paper_grain = grain;
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
            &self.gpu.device,
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
        crate::frame::render(
            &self.gpu,
            &mut self.cell_grid,
            if self.paper_texture {
                Some(&self.paper_bg)
            } else {
                None
            },
            &self.crt,
            self.window_opacity,
            // Newsprint: light themes multiply the user's grain knob
            // (theme().grain = 1.2 on light AND dark; the dark-grain
            // calibration assumes the 1.3 × 1.2 = 1.56 product).
            self.paper_grain * crew_theme::theme().grain,
            panes,
        );
    }
}
