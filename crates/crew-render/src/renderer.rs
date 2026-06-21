use std::sync::Arc;

use winit::window::Window;

use crate::cellgrid::CellGrid;
use crate::gpu::Gpu;
use crate::scene::PaneScene;

/// Top-level renderer: owns `Gpu` + `CellGrid` and orchestrates the full frame.
pub struct Renderer {
    gpu: Gpu,
    cell_grid: CellGrid,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let gpu = Gpu::new(window)?;
        let cell_grid = CellGrid::new(&gpu);
        Ok(Self { gpu, cell_grid })
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.gpu.resize(w, h);
        self.cell_grid.resize(w as f32, h as f32);
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
        self.cell_grid.set_scene(&self.gpu, panes);
        self.cell_grid.prepare(&self.gpu);

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

        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("crew frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 8.0 / 255.0,
                            g: 8.0 / 255.0,
                            b: 16.0 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.cell_grid.draw(&mut pass);
        }

        self.gpu.queue.submit(Some(enc.finish()));
        frame.present();
    }
}
