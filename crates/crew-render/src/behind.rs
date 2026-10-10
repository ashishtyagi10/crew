//! What lies behind liquid glass: the page alone, rendered into a texture of
//! its own before the frame, so the glass pass can sample it — frost it, and
//! bend it through the lens at each card's rim (`glass.wgsl`). The page is a
//! flat colour at the window's opacity since no theme paints a wallpaper
//! (2026-10-09); the glass reads that opacity back from it.
//!
//! Its own pass rather than a copy of the frame: the surface can only be read
//! back where the platform grants it `COPY_SRC`.

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

    /// Draw the page exactly as the frame's own pass begins: premultiplied,
    /// at the window's opacity (`bg` is straight). The glass reads the
    /// opacity back from the alpha (`glass.wgsl`'s `under`).
    pub fn encode(&self, enc: &mut wgpu::CommandEncoder, bg: [f32; 4]) {
        let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("glass behind"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(crate::color::premultiplied(bg)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
}
