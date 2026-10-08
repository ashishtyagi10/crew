//! Instanced frosted-glass card layer (SDF-based, alpha-blended).
//!
//! Drawn first inside the scene pass — under the cell background quads, the
//! rounded borders and the text — so every pane sits on a translucent sheet
//! with the paper grain showing through it. Geometry mirrors
//! [`crate::roundborder`]; the difference is that this fills the shape (with a
//! vertical ramp, a specular top edge and a soft shadow) instead of stroking it.
use wgpu::util::DeviceExt as _;

/// One frosted card to draw.
pub struct GlassCard {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    /// Fill opacity at the top and bottom edges.
    pub alpha_top: f32,
    pub alpha_bottom: f32,
    /// Frost grain amplitude.
    pub noise: f32,
    /// Fill tint, already converted for the target format.
    pub tint: [f32; 4],
    /// Specular hairline colour, already converted for the target format.
    pub highlight: [f32; 4],
    pub highlight_alpha: f32,
    pub shadow_alpha: f32,
    /// Scan-highlight position in `0.0..=1.0`; negative draws no scan.
    pub scan: f32,
    /// Inner edge-glow strength; 0 (paper) must leave the fill untouched.
    pub edge_glow: f32,
    /// The broad glossy reflection across the upper face; 0 draws none.
    pub gloss: f32,
    /// How far the shadow glows in the tint instead of shading black.
    pub glow: f32,
    /// Fill alpha of the raster etched into the glass body; 0 is clear glass.
    pub etch: f32,
    /// Elevation, `-1.0..=2.0`: 0 rests on the page, 1 is the focused lift,
    /// 2 a floating card — each a deeper, wider ambient shadow and a brighter
    /// rim. Down to -1: a well, shadowed inside under its top lip.
    pub lift: f32,
    /// Focus glint position along the rim, `0.0..=1.0`; negative draws none.
    pub glint: f32,
    /// Where the frame's legends break the rim (see [`crate::notch`]).
    pub notch: crate::notch::Notch,
    /// Liquid glass (`crew_theme::LiquidStyle`): refract, bevel, blur and
    /// dispersion, then clear_rim and vibrance, how much of the desktop the
    /// body lets through on a see-through window (0 is a solid slab, as the
    /// glass always was), one spare and an on flag.
    /// All zero draws the sheet as before, never sampling the backdrop.
    pub lens: [f32; 8],
}

/// 48 × f32 per instance: rect(4), params(4), tint(4), highlight(4), extra(4),
/// then the notch depth with the gloss, glow and etch beside it (4), the notch's
/// top spans(8) and bottom spans(8), and the lens (8).
const INSTANCE_FLOATS: usize = 48;

/// GPU layer drawing rounded translucent cards via a signed-distance field.
pub struct GlassLayer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    /// Group 1: what lies behind the glass (`crate::behind`) and how it is
    /// sampled — a 1×1 stand-in until a liquid theme hands over the real one.
    behind_bgl: wgpu::BindGroupLayout,
    behind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    vp_buf: wgpu::Buffer,
    inst_buf: Option<wgpu::Buffer>,
    count: u32,
}

fn f32s_as_bytes(data: &[f32]) -> &[u8] {
    // SAFETY: f32 is Pod (no padding, valid for any bit pattern).
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}

/// Pack one card into its instance floats. Split out so the layout the shader
/// depends on can be asserted without a GPU.
fn pack(c: &GlassCard) -> [f32; INSTANCE_FLOATS] {
    let n = &c.notch;
    let (t, b) = (n.top, n.bottom);
    [
        c.x,
        c.y,
        c.w,
        c.h,
        c.radius,
        c.alpha_top,
        c.alpha_bottom,
        c.noise,
        c.tint[0],
        c.tint[1],
        c.tint[2],
        c.highlight_alpha,
        c.highlight[0],
        c.highlight[1],
        c.highlight[2],
        c.shadow_alpha,
        c.scan,
        c.edge_glow,
        c.lift,
        c.glint,
        n.depth,
        c.gloss,
        c.glow,
        c.etch,
        t[0][0],
        t[0][1],
        t[1][0],
        t[1][1],
        t[2][0],
        t[2][1],
        t[3][0],
        t[3][1],
        b[0][0],
        b[0][1],
        b[1][0],
        b[1][1],
        b[2][0],
        b[2][1],
        b[3][0],
        b[3][1],
        c.lens[0],
        c.lens[1],
        c.lens[2],
        c.lens[3],
        c.lens[4],
        c.lens[5],
        c.lens[6],
        c.lens[7],
    ]
}

impl GlassLayer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glass"),
            source: wgpu::ShaderSource::Wgsl(include_str!("glass.wgsl").into()),
        });

        let vp_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glass_vp"),
            contents: f32s_as_bytes(&[1.0_f32, 1.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glass_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glass_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vp_buf.as_entire_binding(),
            }],
        });

        let behind_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glass_behind_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glass_behind_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let stand_in = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glass_behind_stand_in"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let behind_group = behind_group(
            device,
            &behind_bgl,
            &stand_in.create_view(&Default::default()),
            &sampler,
        );

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glass_layout"),
            bind_group_layouts: &[Some(&bgl), Some(&behind_bgl)],
            immediate_size: 0,
        });

        let inst_attrs = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4,
            5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4,
            10 => Float32x4, 11 => Float32x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glass_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: (INSTANCE_FLOATS * 4) as u64,
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
                    blend: Some(crate::blend::STRAIGHT_OVER),
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
            behind_bgl,
            behind_group,
            sampler,
            vp_buf,
            inst_buf: None,
            count: 0,
        }
    }

    /// Sample `view` as what lies behind the glass (see `crate::behind`).
    pub fn set_behind(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) {
        self.behind_group = behind_group(device, &self.behind_bgl, view, &self.sampler);
    }

    /// Upload cards as instance data.
    pub fn set_cards(&mut self, device: &wgpu::Device, cards: &[GlassCard]) {
        self.count = cards.len() as u32;
        if cards.is_empty() {
            self.inst_buf = None;
            return;
        }
        let mut data: Vec<f32> = Vec::with_capacity(cards.len() * INSTANCE_FLOATS);
        for c in cards {
            data.extend_from_slice(&pack(c));
        }
        self.inst_buf = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("glass_inst"),
                contents: f32s_as_bytes(&data),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
    }

    /// Update the viewport uniform (call on resize), with the light at rest.
    pub fn set_viewport(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        self.set_view(queue, width, height, (0.0, 0.0));
    }

    /// The viewport plus the rims' light tilt, `-1..=1` per axis.
    pub fn set_view(&self, queue: &wgpu::Queue, width: f32, height: f32, tilt: (f32, f32)) {
        queue.write_buffer(
            &self.vp_buf,
            0,
            f32s_as_bytes(&[width, height, tilt.0, tilt.1]),
        );
    }

    /// Record draw commands into an active render pass.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let Some(ref buf) = self.inst_buf else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_bind_group(1, &self.behind_group, &[]);
        pass.set_vertex_buffer(0, buf.slice(..));
        pass.draw(0..6, 0..self.count);
    }
}

fn behind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("glass_behind_bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

#[cfg(test)]
#[path = "glass_tests.rs"]
mod tests;
