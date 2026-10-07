//! What lies behind liquid glass: the wallpaper alone, rendered into a texture
//! of its own before the frame, so the glass pass can sample it — frost it,
//! and bend it through the lens at each card's rim (`glass.wgsl`).
//!
//! Its own pass rather than a copy of the frame: the surface can only be read
//! back where the platform grants it `COPY_SRC`, and the wallpaper is one
//! full-screen triangle either way. Only a liquid theme pays for it — every
//! other theme never creates the texture (see `Renderer::frame`).
use crate::paperbg::PaperBgPass;

/// The wallpaper target: the frame's size and format.
pub struct Behind {
    pub view: wgpu::TextureView,
    width: u32,
    height: u32,
}

impl Behind {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glass_behind"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        Self {
            view: texture.create_view(&Default::default()),
            width,
            height,
        }
    }

    /// Whether the target already matches `(w, h)`.
    pub fn matches(&self, w: u32, h: u32) -> bool {
        self.width == w.max(1) && self.height == h.max(1)
    }

    /// Draw the wallpaper — the page colour and, with the paper texture on,
    /// the backdrop pass over it — exactly as the frame's own pass begins.
    pub fn encode(
        &self,
        enc: &mut wgpu::CommandEncoder,
        bg: [f32; 4],
        paper: Option<&PaperBgPass>,
    ) {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("glass behind"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: bg[0] as f64,
                        g: bg[1] as f64,
                        b: bg[2] as f64,
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
        if let Some(paper) = paper {
            paper.draw(&mut pass);
        }
    }
}
