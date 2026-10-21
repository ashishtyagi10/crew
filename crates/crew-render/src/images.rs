//! The layer pictures are drawn on (`picture`): a texture per picture,
//! uploaded the first frame it shows and kept while it goes on showing, and
//! one textured rectangle per picture each frame, over the cells' own
//! backgrounds and under their text.
use std::collections::HashMap;
use std::sync::Arc;

use wgpu::util::DeviceExt as _;

use crate::picture::{Pixels, Placed};
use crate::postfx::f32s_as_bytes;

/// Frames a texture outlives the last frame that drew it: a picture
/// scrolled out of view and back is not uploaded again.
const KEEP_FRAMES: u64 = 600;

struct Tex {
    _texture: wgpu::Texture,
    bind: wgpu::BindGroup,
    used: u64,
}

pub(crate) struct ImageLayer {
    pipeline: wgpu::RenderPipeline,
    vp_buf: wgpu::Buffer,
    vp_bind: wgpu::BindGroup,
    tex_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// The colour pictures arrive in (sRGB) as the target reads it: an sRGB
    /// target samples them linear, any other takes the bytes as they are.
    format: wgpu::TextureFormat,
    textures: HashMap<u64, Tex>,
    frame: u64,
    /// This frame's pictures, in drawing order, by id.
    draws: Vec<u64>,
    inst_buf: Option<wgpu::Buffer>,
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

impl ImageLayer {
    pub(crate) fn new(device: &wgpu::Device, target: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("images"),
            source: wgpu::ShaderSource::Wgsl(include_str!("images.wgsl").into()),
        });
        let vp_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("images_vp"),
            contents: f32s_as_bytes(&[1.0_f32, 1.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let vp_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("images_vp_bgl"),
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
        let vp_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("images_vp_bg"),
            layout: &vp_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vp_buf.as_entire_binding(),
            }],
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("images_tex_bgl"),
            entries: &[
                texture_entry(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("images_layout"),
            bind_group_layouts: &[Some(&vp_layout), Some(&tex_layout)],
            immediate_size: 0,
        });
        let inst_attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("images_pipeline"),
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
                    format: target,
                    blend: Some(crate::blend::PREMUL_OVER),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let format = match target.is_srgb() {
            true => wgpu::TextureFormat::Rgba8UnormSrgb,
            false => wgpu::TextureFormat::Rgba8Unorm,
        };
        Self {
            pipeline,
            vp_buf,
            vp_bind,
            tex_layout,
            sampler: crate::postfx::sampler(device, "images_sampler"),
            format,
            textures: HashMap::new(),
            frame: 0,
            draws: Vec::new(),
            inst_buf: None,
        }
    }

    /// This frame's pictures: each one's texture made if it is new, the
    /// rectangles uploaded, and textures nothing has drawn in a while let go.
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        placed: &[Placed],
        (width, height): (f32, f32),
    ) {
        self.frame += 1;
        queue.write_buffer(&self.vp_buf, 0, f32s_as_bytes(&[width, height, 0.0, 0.0]));
        let mut data: Vec<f32> = Vec::with_capacity(placed.len() * 8);
        self.draws.clear();
        for p in placed {
            let id = p.pixels.id();
            if !self.textures.contains_key(&id) {
                let tex = self.upload(device, queue, &p.pixels);
                self.textures.insert(id, tex);
            }
            if let Some(t) = self.textures.get_mut(&id) {
                t.used = self.frame;
            }
            self.draws.push(id);
            data.extend_from_slice(&p.rect);
            data.extend_from_slice(&p.uv);
        }
        let frame = self.frame;
        self.textures
            .retain(|_, t| frame.saturating_sub(t.used) <= KEEP_FRAMES);
        self.inst_buf = (!data.is_empty()).then(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("images_inst"),
                contents: f32s_as_bytes(&data),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
    }

    fn upload(&self, device: &wgpu::Device, queue: &wgpu::Queue, px: &Arc<Pixels>) -> Tex {
        let size = wgpu::Extent3d {
            width: px.w.max(1),
            height: px.h.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("picture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        if px.rgba.len() as u64 >= u64::from(size.width) * u64::from(size.height) * 4 {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &px.rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size.width * 4),
                    rows_per_image: Some(size.height),
                },
                size,
            );
        }
        let view = texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("picture_bg"),
            layout: &self.tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        Tex {
            _texture: texture,
            bind,
            used: self.frame,
        }
    }

    pub(crate) fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let Some(buf) = &self.inst_buf else { return };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.vp_bind, &[]);
        pass.set_vertex_buffer(0, buf.slice(..));
        for (i, id) in self.draws.iter().enumerate() {
            if let Some(t) = self.textures.get(id) {
                pass.set_bind_group(1, &t.bind, &[]);
                pass.draw(0..6, i as u32..i as u32 + 1);
            }
        }
    }
}
