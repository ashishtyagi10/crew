//! The CRT composite pass: samples the off-screen scene texture (see
//! [`crate::scenetarget::SceneTarget`]) plus the blurred bloom (see
//! [`crate::bloom::Bloom`]) and draws them to the surface through `crt.wgsl`.
//! The look knobs are no longer module constants — each theme ships its own
//! [`crew_theme::CrtStyle`], written per frame by [`CrtPass::update_uniform`].
//! The bind group references both textures, so it is (re)built via
//! [`CrtPass::set_source`] whenever the targets are created or resized.
use crate::postfx;

pub struct CrtPass {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buf: wgpu::Buffer,
    bind_group: Option<wgpu::BindGroup>,
}

impl CrtPass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("crt"),
            source: wgpu::ShaderSource::Wgsl(include_str!("crt.wgsl").into()),
        });
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crt_uniform"),
            size: 32, // 8 × f32
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // `true`: the composite reads a second texture — the bloom — at @3.
        let bgl = postfx::layout(device, "crt_bgl", true);
        Self {
            pipeline: postfx::pipeline(device, &bgl, &shader, "fs", format),
            bgl,
            sampler: postfx::sampler(device, "crt_sampler"),
            uniform_buf,
            bind_group: None,
        }
    }

    /// (Re)build the bind group against the scene-target and blurred-bloom
    /// views. Call once after the targets are created and again whenever they
    /// are recreated (resize).
    pub fn set_source(
        &mut self,
        device: &wgpu::Device,
        scene: &wgpu::TextureView,
        bloom: &wgpu::TextureView,
    ) {
        self.bind_group = Some(postfx::bind_group(
            device,
            "crt_bg",
            &self.bgl,
            &[scene, bloom],
            &self.sampler,
            &self.uniform_buf,
        ));
    }

    /// Write the per-frame uniform. `time` advances only while animating;
    /// `flicker` is 0 when idle (making the pass fully static). Everything
    /// else is the theme's own tube tuning.
    pub fn update_uniform(
        &self,
        queue: &wgpu::Queue,
        width: f32,
        height: f32,
        time: f32,
        flicker: f32,
        style: crew_theme::CrtStyle,
    ) {
        let data: [f32; 8] = [
            width,
            height,
            time,
            flicker,
            style.curvature,
            style.scanline,
            style.glow,
            style.corner,
        ];
        queue.write_buffer(&self.uniform_buf, 0, postfx::f32s_as_bytes(&data));
    }

    /// Encode the composite onto `view`. No-op until `set_source` has
    /// supplied the textures.
    pub fn encode(&self, enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let Some(bg) = &self.bind_group else {
            return;
        };
        postfx::run(enc, "crt", view, &self.pipeline, bg);
    }
}
