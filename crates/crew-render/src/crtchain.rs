//! The whole CRT post-process, owned as one unit: the off-screen scene
//! target, the half-res bloom chain, and the final composite pass. The
//! renderer's frame path and the headless GPU test both drive THIS type, so
//! what the test measures is the real composite — bloom included — not a
//! test-only re-plumbing of the passes.
use crate::bloom::Bloom;
use crate::crt::CrtPass;
use crate::scenetarget::SceneTarget;
use crew_theme::CrtStyle;

pub struct CrtChain {
    pass: CrtPass,
    bloom: Bloom,
    target: SceneTarget,
    format: wgpu::TextureFormat,
    style: Option<CrtStyle>,
    time: f32,
    flicker: f32,
}

impl CrtChain {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, w: u32, h: u32) -> Self {
        let mut chain = Self {
            pass: CrtPass::new(device, format),
            bloom: Bloom::new(device),
            target: SceneTarget::new(device, format, w, h),
            format,
            style: None,
            time: 0.0,
            flicker: 0.0,
        };
        chain.bind(device, w, h);
        chain
    }

    /// Wire bloom to the scene target and the composite to both outputs.
    fn bind(&mut self, device: &wgpu::Device, w: u32, h: u32) {
        self.bloom.set_source(device, &self.target.view, w, h);
        // `bind` always runs right after `bloom.set_source`, so output exists.
        let bloom_view = self.bloom.output().expect("bloom targets just built");
        self.pass.set_source(device, &self.target.view, bloom_view);
    }

    /// Track the surface size: recreate the scene target and bloom targets
    /// (and rebind everything) only when the size actually changed.
    pub fn resize(&mut self, device: &wgpu::Device, w: u32, h: u32) {
        if self.target.matches(w, h) {
            return;
        }
        self.target = SceneTarget::new(device, self.format, w, h);
        self.bind(device, w, h);
    }

    /// The off-screen view the scene passes draw into while CRT is on.
    pub fn scene_view(&self) -> &wgpu::TextureView {
        &self.target.view
    }

    /// The scene texture itself — the headless harness writes its source
    /// patterns straight into the real target instead of a stand-in.
    pub fn scene_texture(&self) -> &wgpu::Texture {
        &self.target.texture
    }

    /// The active tube tuning; `None` turns the whole chain off.
    pub fn set_style(&mut self, style: Option<CrtStyle>) {
        self.style = style;
    }

    pub fn style(&self) -> Option<CrtStyle> {
        self.style
    }

    /// Per-frame animation inputs: `time` seeds the flicker hash, `flicker`
    /// is its amplitude (0 = a static tube).
    pub fn set_anim(&mut self, time: f32, flicker: f32) {
        self.time = time;
        self.flicker = flicker;
    }

    /// Write the frame's uniforms (composite + bloom) from the active style.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, w: f32, h: f32) {
        let style = self.style.unwrap_or(CrtStyle::DEFAULT);
        self.pass
            .update_uniform(queue, w, h, self.time, self.flicker, style);
        self.bloom.update_uniform(queue, style.glow_radius);
    }

    /// Encode the full post-process: three bloom passes over the scene, then
    /// the composite onto `surface_view`.
    pub fn encode(&self, enc: &mut wgpu::CommandEncoder, surface_view: &wgpu::TextureView) {
        self.bloom.encode(enc);
        self.pass.encode(enc, surface_view);
    }
}
