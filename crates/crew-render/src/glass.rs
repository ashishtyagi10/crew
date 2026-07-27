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
}

/// 16 × f32 per instance: rect(4), params(4), tint(4), highlight(4).
const INSTANCE_FLOATS: usize = 16;

/// GPU layer drawing rounded translucent cards via a signed-distance field.
pub struct GlassLayer {
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

/// Pack one card into its instance floats. Split out so the layout the shader
/// depends on can be asserted without a GPU.
fn pack(c: &GlassCard) -> [f32; INSTANCE_FLOATS] {
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
            label: Some("glass_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vp_buf.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glass_layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let inst_attrs = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4];
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

    /// Update the viewport uniform (call on resize).
    pub fn set_viewport(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        queue.write_buffer(&self.vp_buf, 0, f32s_as_bytes(&[width, height, 0.0, 0.0]));
    }

    /// Record draw commands into an active render pass.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let Some(ref buf) = self.inst_buf else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, buf.slice(..));
        pass.draw(0..6, 0..self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> GlassCard {
        GlassCard {
            x: 10.0,
            y: 20.0,
            w: 300.0,
            h: 200.0,
            radius: 10.0,
            alpha_top: 0.2,
            alpha_bottom: 0.09,
            noise: 0.012,
            tint: [0.1, 0.2, 0.3, 1.0],
            highlight: [1.0, 1.0, 1.0, 1.0],
            highlight_alpha: 0.22,
            shadow_alpha: 0.3,
        }
    }

    /// The shader reads these by fixed offset; a reordering here would show up
    /// as garbled cards rather than a compile error, so pin the layout.
    #[test]
    fn packing_matches_the_shader_layout() {
        let p = pack(&card());
        assert_eq!(&p[0..4], &[10.0, 20.0, 300.0, 200.0], "rect");
        assert_eq!(&p[4..8], &[10.0, 0.2, 0.09, 0.012], "radius/alphas/noise");
        assert_eq!(
            &p[8..12],
            &[0.1, 0.2, 0.3, 0.22],
            "tint.rgb + highlight_alpha"
        );
        assert_eq!(
            &p[12..16],
            &[1.0, 1.0, 1.0, 0.3],
            "highlight.rgb + shadow_alpha"
        );
    }

    /// The vertex buffer stride must match what `pack` produces, or every
    /// instance after the first reads misaligned data.
    #[test]
    fn stride_matches_the_packed_size() {
        assert_eq!(pack(&card()).len(), INSTANCE_FLOATS);
    }

    /// The quad is expanded by PAD in the shader so the blurred shadow is not
    /// clipped to a hard square. Keep that padding ahead of the falloff it has
    /// to contain (blur + drop), which this asserts against the shader source.
    #[test]
    fn shadow_padding_covers_its_falloff() {
        let src = include_str!("glass.wgsl");
        let num = |name: &str| -> f32 {
            let at = src
                .find(&format!("const {name}: f32 = "))
                .unwrap_or_else(|| panic!("{name} missing from glass.wgsl"));
            let rest = &src[at + format!("const {name}: f32 = ").len()..];
            let end = rest.find(';').expect("unterminated const");
            rest[..end].trim().parse().expect("non-numeric const")
        };
        let (pad, blur, drop) = (num("PAD"), num("SH_BLUR"), num("SH_DROP"));
        assert!(
            pad >= blur + drop,
            "PAD {pad} cannot contain a {blur}px blur dropped {drop}px"
        );
    }
}
