//! What a sheer window keeps solid: a handful of alpha-only rects, drawn last
//! in the scene pass.
//!
//! A sheer window is cleared at the user's opacity and everything crew draws
//! blends over it, so the desktop comes through EVERYTHING at once — the card
//! being read and crew's own furniture along with it. This pass hands the
//! opacity back where it is not wanted: it writes `alpha = 1` inside each rect
//! the frame names and touches nothing else, with `ColorWrites::ALPHA` and no
//! blend state.
//!
//! The rects are the focused card (see [`crate::scene::focused_card_rect`])
//! and crew's chrome — the input bar and the left nav, handed over by the app,
//! which is where the layout lives. Transparency is for the canvas and the
//! cards you are not reading; the bar you type into is not scenery.
//!
//! Alpha-only is the whole point. A page-coloured sheet under the cells would
//! have worked on a flat theme and flattened the modern family's backdrop on
//! every other one — the gradient wash gathers its light on the focused card
//! (see crew-app's `washfocus`), which is exactly the pixels such a sheet
//! would paint over. Writing the channel that carries translucency leaves the
//! wash, the lattice, the grain and the text precisely as they were.
use wgpu::util::DeviceExt as _;

fn f32s_as_bytes(data: &[f32]) -> &[u8] {
    // SAFETY: f32 is Pod (no padding, valid for any bit pattern).
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}

/// How many rects one frame may solidify. The frame asks for at most three
/// (the focused card, the input bar, the nav); the slack is for a look that
/// wants a couple more without a shader change.
pub const MAX_SOLID_RECTS: usize = 8;

/// Floats in the uniform: `res` (4) + [`MAX_SOLID_RECTS`] × 4.
const UNIFORM_FLOATS: usize = 4 + MAX_SOLID_RECTS * 4;

/// Pass that solidifies a few rectangles of the frame, alpha only.
pub struct SolidCardPass {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    /// Rects the last [`Self::set_rects`] accepted — the instance count.
    count: u32,
}

impl SolidCardPass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("solidcard"),
            source: wgpu::ShaderSource::Wgsl(include_str!("solidcard.wgsl").into()),
        });

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("solidcard_uniform"),
            contents: f32s_as_bytes(&[0.0f32; UNIFORM_FLOATS]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("solidcard_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                // The vertex stage reads the rects (it builds a quad per
                // instance); the fragment stage reads nothing at all.
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("solidcard_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("solidcard_layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("solidcard_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[], // fullscreen triangle — no vertex buffer
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // No blend: the alpha written REPLACES what is there. The
                    // write mask is what keeps the colour underneath intact —
                    // without it this pass would paint the card black.
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALPHA,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group,
            uniform_buf,
            count: 0,
        }
    }

    /// Set the rects to solidify — `[x, y, w, h]` in physical px — against a
    /// surface of `(width, height)`. Empty rects are dropped, and anything
    /// past [`MAX_SOLID_RECTS`] is ignored rather than silently wrapping onto
    /// slot zero.
    pub fn set_rects(&mut self, queue: &wgpu::Queue, rects: &[[f32; 4]], (w, h): (f32, f32)) {
        let mut data = [0.0f32; UNIFORM_FLOATS];
        data[0] = w;
        data[1] = h;
        let mut n = 0;
        for r in rects.iter().filter(|r| r[2] > 0.0 && r[3] > 0.0) {
            if n == MAX_SOLID_RECTS {
                break;
            }
            data[4 + n * 4..4 + n * 4 + 4].copy_from_slice(r);
            n += 1;
        }
        self.count = n as u32;
        queue.write_buffer(&self.uniform_buf, 0, f32s_as_bytes(&data));
    }

    /// Whether the last [`Self::set_rects`] left anything to draw.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..6, 0..self.count);
    }
}
