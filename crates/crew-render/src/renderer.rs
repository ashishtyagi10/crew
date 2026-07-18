use std::sync::Arc;

use winit::window::Window;

use crate::cellgrid::CellGrid;
use crate::crt::CrtPass;
use crate::gpu::Gpu;
use crate::paperbg::PaperBgPass;
use crate::scene::PaneScene;
use crate::scenetarget::SceneTarget;

/// Top-level renderer: owns `Gpu` + `CellGrid` and orchestrates the full frame.
pub struct Renderer {
    gpu: Gpu,
    cell_grid: CellGrid,
    paper_bg: PaperBgPass,
    paper_texture: bool,
    paper_grain: f32,
    // CRT post-process: when `crt_on`, render into `scene_target` then
    // reproject through `crt`; otherwise the frame draws straight to the surface.
    crt: CrtPass,
    scene_target: SceneTarget,
    crt_on: bool,
    crt_time: f32,
    crt_flicker: f32,
}

impl Renderer {
    pub fn new(window: Arc<Window>, font_size: f32) -> anyhow::Result<Self> {
        let gpu = Gpu::new(window)?;
        let cell_grid = CellGrid::new(&gpu.device, &gpu.queue, gpu.format, font_size);
        let paper_bg = PaperBgPass::new(&gpu.device, gpu.format);
        let mut crt = CrtPass::new(&gpu.device, gpu.format);
        let scene_target =
            SceneTarget::new(&gpu.device, gpu.format, gpu.config.width, gpu.config.height);
        crt.set_source(&gpu.device, &scene_target.view);
        Ok(Self {
            gpu,
            cell_grid,
            paper_bg,
            paper_texture: true,
            // Matches config's default_paper_grain; the app calls set_paper_grain
            // right after construction, so this is just a sane standalone default.
            paper_grain: 1.3,
            crt,
            scene_target,
            crt_on: false,
            crt_time: 0.0,
            crt_flicker: 0.0,
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

    /// Set the grain amplitude multiplier (0.0 = no grain, 1.0 = default ~±3%, 2.0 = double).
    /// This stores the USER knob only; the active theme's `grain`
    /// multiplies it at frame time in `frame()`, so light themes render
    /// noticeably grainier newsprint without changing what's stored here.
    pub fn set_paper_grain(&mut self, grain: f32) {
        self.paper_grain = grain;
    }

    /// Turn the CRT tube post-process on or off. When off, the frame draws
    /// straight to the surface with no extra pass (the original path).
    pub fn set_crt(&mut self, on: bool) {
        self.crt_on = on;
    }

    /// Whether the CRT post-process is currently active.
    pub fn crt_on(&self) -> bool {
        self.crt_on
    }

    /// Per-frame CRT animation: `time` seeds the flicker hash, `flicker` is its
    /// amplitude (0 = a static tube). The app lifts these only while streaming.
    pub fn set_crt_anim(&mut self, time: f32, flicker: f32) {
        self.crt_time = time;
        self.crt_flicker = flicker;
    }

    /// Sorted, de-duplicated names of all installed monospace font families.
    pub fn monospace_families(&mut self) -> Vec<String> {
        self.cell_grid.monospace_families()
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.gpu.resize(w, h);
        self.cell_grid.resize(w as f32, h as f32);
        // The off-screen CRT target tracks the surface size.
        if !self
            .scene_target
            .matches(self.gpu.config.width, self.gpu.config.height)
        {
            self.scene_target = SceneTarget::new(
                &self.gpu.device,
                self.gpu.format,
                self.gpu.config.width,
                self.gpu.config.height,
            );
            self.crt
                .set_source(&self.gpu.device, &self.scene_target.view);
        }
    }

    /// Returns the monospace cell size `(width, height)` in pixels.
    pub fn cell_size(&self) -> (f32, f32) {
        self.cell_grid.cell_size()
    }

    /// Returns the current surface dimensions `(width, height)` in pixels.
    pub fn surface_size(&self) -> (u32, u32) {
        (self.gpu.config.width, self.gpu.config.height)
    }

    /// Upload a scene of panes, render, and present the frame.
    /// Skips the frame on surface errors (Outdated/Lost).
    pub fn frame(&mut self, panes: &[PaneScene]) {
        self.cell_grid.set_scene(&self.gpu.device, panes);
        self.cell_grid.prepare(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.config.width,
            self.gpu.config.height,
        );

        let frame = match self.gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost
            | wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("surface lost/outdated/validation — skipping frame");
                return;
            }
        };

        let view = frame.texture.create_view(&Default::default());
        let mut enc = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // CRT on → scene renders off-screen then reprojects; off → straight to
        // the surface (the original, zero-overhead path). See `frame::encode`.
        let use_crt = self.crt_on;
        let bg = crew_theme::theme().page_bg;
        let bg_f32 = crate::color::target_rgba(bg, 1.0, self.gpu.format.is_srgb());

        if self.paper_texture {
            self.paper_bg.update_uniform(
                &self.gpu.queue,
                bg_f32,
                self.gpu.config.width as f32,
                self.gpu.config.height as f32,
                1.0,
                // Newsprint: light themes multiply the user's grain knob
                // (theme().grain = 1.2 there, 1.0 on darks).
                self.paper_grain * crew_theme::theme().grain,
            );
        }
        if use_crt {
            self.crt.update_uniform(
                &self.gpu.queue,
                self.gpu.config.width as f32,
                self.gpu.config.height as f32,
                self.crt_time,
                self.crt_flicker,
            );
        }

        let scene_view = if use_crt {
            &self.scene_target.view
        } else {
            &view
        };
        crate::frame::encode(
            &mut enc,
            &view,
            scene_view,
            use_crt,
            bg_f32,
            if self.paper_texture {
                Some(&self.paper_bg)
            } else {
                None
            },
            &self.cell_grid,
            &self.crt,
        );

        self.gpu.queue.submit(Some(enc.finish()));
        frame.present();
    }
}
