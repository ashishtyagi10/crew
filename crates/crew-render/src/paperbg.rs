use wgpu::util::DeviceExt as _;

fn f32s_as_bytes(data: &[f32]) -> &[u8] {
    // SAFETY: f32 is Pod (no padding, valid for any bit pattern).
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}

/// The modern family's backdrop, drawn behind everything by the background
/// pass: a broad two-pool gradient WASH of theme light, with the fine dot
/// LATTICE woven on top of it. Both ride the same pair of pole colours —
/// already in the target's colour space (the caller runs them through
/// `target_rgba`, like the page colour) — so the page reads as one material.
/// `dots` and `wash` are independent mix weights toward the poles; either at
/// 0 switches its layer off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModernPaper {
    pub color_a: [f32; 3],
    pub color_b: [f32; 3],
    /// Dot lattice strength (0 = no lattice).
    pub dots: f32,
    /// Lattice pitch in physical px (x, y) — the caller derives it from the
    /// cell metrics so the grid rides font size and DPI.
    pub spacing: [f32; 2],
    /// Dot radius in physical px.
    pub radius: f32,
    /// Gradient wash strength (0 = no wash): the mix weight at each pool's
    /// centre, falling to nothing between them.
    pub wash: f32,
    /// Where the two pools sit on their orbit, in turns: 0 puts `color_a` at
    /// the left edge and `color_b` at the right, 0.25 rotates them a quarter
    /// turn clockwise. The app advances it only while a pane is busy, so an
    /// idle frame is a pure function of pixel position (see crew-app's
    /// `washphase`).
    pub phase: f32,
}

impl ModernPaper {
    /// Lattice geometry — `(spacing, radius)` in physical px — for a text row
    /// of `cell_h` px. The grid is SQUARE and fine: roughly six dots to a text
    /// row, so it reads as woven engineering paper rather than a sparse polka
    /// grid, with pin-fine dots that never touch. Deriving it from the cell
    /// keeps the texture riding font size and DPI; the clamps keep tiny and
    /// huge fonts inside a pitch that still renders cleanly.
    pub fn cell_geometry(cell_h: f32) -> ([f32; 2], f32) {
        let pitch = (cell_h / 6.0).clamp(5.0, 9.0);
        ([pitch, pitch], (pitch * 0.2).clamp(0.8, 1.4))
    }
}

/// Full-screen background pass: fills the surface with `page_bg` modulated by
/// subtle procedural grain and a faint radial vignette.
pub struct PaperBgPass {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
}

impl PaperBgPass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("paperbg"),
            source: wgpu::ShaderSource::Wgsl(include_str!("paperbg.wgsl").into()),
        });

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("paperbg_uniform"),
            contents: f32s_as_bytes(&[0.0f32; 20]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("paperbg_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("paperbg_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("paperbg_layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("paperbg_pipeline"),
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
                    blend: None, // opaque — replaces the clear colour
                    write_mask: wgpu::ColorWrites::ALL,
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
        }
    }

    /// Write the per-frame uniform: theme background colour, surface resolution,
    /// effect intensity (1.0 = full grain+vignette, 0.0 = flat fill), grain
    /// amplitude multiplier (0.0 = no grain, 1.0 = default ~±3%, 2.0 = double)
    /// and the modern-family backdrop (`None` = neither wash nor dots).
    pub fn update_uniform(
        &self,
        queue: &wgpu::Queue,
        page_bg: [f32; 4],
        // Surface `(width, height)` in px — one parameter, because they are
        // never meaningful apart.
        (width, height): (f32, f32),
        intensity: f32,
        grain_mul: f32,
        modern: Option<&ModernPaper>,
    ) {
        let d = modern.copied().unwrap_or(ModernPaper {
            color_a: [0.0; 3],
            color_b: [0.0; 3],
            dots: 0.0,
            spacing: [1.0; 2],
            radius: 0.0,
            wash: 0.0,
            phase: 0.0,
        });
        let data: [f32; 20] = [
            page_bg[0],
            page_bg[1],
            page_bg[2],
            page_bg[3],
            width,
            height,
            intensity,
            grain_mul,
            d.color_a[0],
            d.color_a[1],
            d.color_a[2],
            d.dots,
            d.color_b[0],
            d.color_b[1],
            d.color_b[2],
            d.radius,
            d.spacing[0],
            d.spacing[1],
            d.wash,
            d.phase,
        ];
        queue.write_buffer(&self.uniform_buf, 0, f32s_as_bytes(&data));
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
#[path = "paperbg_tests.rs"]
mod tests;
