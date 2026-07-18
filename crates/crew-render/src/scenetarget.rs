//! Off-screen colour target for the CRT post-process. When CRT is active the
//! frame is rendered into this texture (exactly the passes that otherwise draw
//! straight to the surface — same format, so glyph output is identical), then
//! the `CrtPass` samples it onto the real surface as a flat panel with
//! scanlines and phosphor glow. Recreated on resize; `view` feeds the CRT bind
//! group.
pub struct SceneTarget {
    pub view: wgpu::TextureView,
    width: u32,
    height: u32,
}

impl SceneTarget {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("crt_scene_target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            // RENDER_ATTACHMENT: the frame draws into it. TEXTURE_BINDING: the
            // CRT pass samples it. COPY_SRC lets the headless harness read back.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self {
            view,
            width,
            height,
        }
    }

    /// Whether the target already matches `(w, h)` — lets the renderer skip
    /// recreating it (and rebinding the CRT pass) when the size is unchanged.
    pub fn matches(&self, w: u32, h: u32) -> bool {
        self.width == w.max(1) && self.height == h.max(1)
    }
}
