//! The CRT bloom chain: bright-pass downsample to half resolution, then a
//! separable gaussian blur (horizontal, vertical) over two half-res ping-pong
//! targets — see `bloom.wgsl`. `crt.wgsl` samples the blurred result
//! bilinearly and adds it over the scene, which is what replaced the old
//! two-ring neighbour tap whose halo died ~8px out. Rgba16Float targets keep
//! the >1.0 energy the lifted-tail kernel produces instead of clamping it
//! away between passes (float16 is filterable/renderable in core WebGPU, so
//! no extra features are needed). Rebuilt via [`Bloom::set_source`] whenever
//! the scene target is (re)created.
use crate::postfx;

const THRESHOLD: f32 = 0.35;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Half-res targets + bind groups; recreated together with the scene target.
struct Targets {
    half: (u32, u32),
    /// The blur ends here (bright → a, blur H a → b, blur V b → a), so `a`
    /// is also what the CRT composite binds.
    view_a: wgpu::TextureView,
    view_b: wgpu::TextureView,
    bg_bright: wgpu::BindGroup,
    bg_h: wgpu::BindGroup,
    bg_v: wgpu::BindGroup,
}

pub(crate) struct Bloom {
    bright: wgpu::RenderPipeline,
    blur_h: wgpu::RenderPipeline,
    blur_v: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    targets: Option<Targets>,
}

impl Bloom {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bloom.wgsl").into()),
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom_uniform"),
            size: 32, // 8 × f32 (5 used, padded to a 16-byte multiple)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = postfx::layout(device, "bloom_bgl", false);
        Self {
            bright: postfx::pipeline(device, &bgl, &shader, "fs_bright", FORMAT),
            blur_h: postfx::pipeline(device, &bgl, &shader, "fs_blur_h", FORMAT),
            blur_v: postfx::pipeline(device, &bgl, &shader, "fs_blur_v", FORMAT),
            bgl,
            sampler: postfx::sampler(device, "bloom_sampler"),
            uniform,
            targets: None,
        }
    }

    /// (Re)build the half-res targets and bind groups against a scene view of
    /// `w × h` pixels. Mirrors `SceneTarget`: once at creation, again on resize.
    pub fn set_source(&mut self, device: &wgpu::Device, scene: &wgpu::TextureView, w: u32, h: u32) {
        let half = (w.max(1).div_ceil(2), h.max(1).div_ceil(2));
        let tex = |label| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: half.0,
                        height: half.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let (view_a, view_b) = (tex("bloom_a"), tex("bloom_b"));
        let bg = |label, src: &wgpu::TextureView| {
            postfx::bind_group(
                device,
                label,
                &self.bgl,
                &[src],
                &self.sampler,
                &self.uniform,
            )
        };
        self.targets = Some(Targets {
            half,
            bg_bright: bg("bloom_bg_bright", scene),
            bg_h: bg("bloom_bg_h", &view_a),
            bg_v: bg("bloom_bg_v", &view_b),
            view_a,
            view_b,
        });
    }

    /// The blurred bright-pass the CRT composite samples. `None` until
    /// [`Bloom::set_source`] has run.
    pub fn output(&self) -> Option<&wgpu::TextureView> {
        self.targets.as_ref().map(|t| &t.view_a)
    }

    /// Write the per-frame uniform; `radius` is the theme's `glow_radius`
    /// (half-res pixels). Nothing here depends on time — an idle tube reruns
    /// the chain bit-identically.
    /// `ink` switches the bright pass from "what is brighter than the floor"
    /// to "what is more colourful than the page" — the light-page halo (see
    /// `bloom.wgsl`).
    pub fn update_uniform(&self, queue: &wgpu::Queue, radius: f32, ink: bool) {
        let Some(t) = &self.targets else { return };
        let data: [f32; 8] = [
            1.0 / t.half.0 as f32,
            1.0 / t.half.1 as f32,
            radius,
            THRESHOLD,
            f32::from(u8::from(ink)),
            0.0,
            0.0,
            0.0,
        ];
        queue.write_buffer(&self.uniform, 0, postfx::f32s_as_bytes(&data));
    }

    /// Encode the three passes: bright → a, blur H a → b, blur V b → a.
    pub fn encode(&self, enc: &mut wgpu::CommandEncoder) {
        let Some(t) = &self.targets else { return };
        postfx::run(enc, "bloom_bright", &t.view_a, &self.bright, &t.bg_bright);
        postfx::run(enc, "bloom_blur_h", &t.view_b, &self.blur_h, &t.bg_h);
        postfx::run(enc, "bloom_blur_v", &t.view_a, &self.blur_v, &t.bg_v);
    }
}
