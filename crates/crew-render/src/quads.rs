use wgpu::util::DeviceExt as _;

/// A colored rectangle in pixel coordinates.
pub struct Quad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
}

/// GPU layer that draws instanced colored rectangles.
pub struct QuadLayer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vp_buf: wgpu::Buffer,
    inst_buf: Option<wgpu::Buffer>,
    count: u32,
}

fn f32s_as_bytes(data: &[f32]) -> &[u8] {
    // SAFETY: f32 is Pod (no padding, valid for any bit pattern).
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}

impl QuadLayer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quads"),
            source: wgpu::ShaderSource::Wgsl(include_str!("quads.wgsl").into()),
        });

        // Viewport uniform buffer (16 bytes: vec2 + pad vec2).
        let vp_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quads_vp"),
            contents: f32s_as_bytes(&[1.0_f32, 1.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("quads_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
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
            label: Some("quads_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vp_buf.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quads_layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let inst_attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quads_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 8 * 4,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &inst_attrs,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // Alpha blending, not REPLACE: cell backgrounds and rules
                    // are all opaque (identical under either mode), but the
                    // paint layer draws translucent chart fills over them.
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
            vp_buf,
            inst_buf: None,
            count: 0,
        }
    }

    /// Upload quads as instance data. Each quad becomes 8 × f32.
    pub fn set_quads(&mut self, device: &wgpu::Device, quads: &[Quad]) {
        self.count = quads.len() as u32;
        if quads.is_empty() {
            self.inst_buf = None;
            return;
        }

        let mut data: Vec<f32> = Vec::with_capacity(quads.len() * 8);
        for q in quads {
            data.extend_from_slice(&[q.x, q.y, q.w, q.h]);
            data.extend_from_slice(&q.color);
        }

        self.inst_buf = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("quads_inst"),
                contents: f32s_as_bytes(&data),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
    }

    /// Update the viewport uniform (call on resize).
    pub fn set_viewport(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        queue.write_buffer(&self.vp_buf, 0, f32s_as_bytes(&[width, height, 0.0, 0.0]));
    }

    /// Record draw commands into an active render pass.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let Some(ref buf) = self.inst_buf else { return };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, buf.slice(..));
        pass.draw(0..6, 0..self.count);
    }
}
