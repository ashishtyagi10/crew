//! Theme-switch crossfade: a persistent snapshot of the last presented frame
//! plus the pass that draws it back over the next frames at a decaying
//! opacity. While no fade runs, every frame is copied surface → snapshot
//! (one GPU blit); the moment the theme flips, the copy stops, so the
//! snapshot holds the final OLD-theme frame and the new theme renders fully
//! underneath it from its first frame — the old look melts away with no
//! blank wash at any point. When the surface can't be a copy source (no
//! `COPY_SRC` in the surface caps) the pass stays inert and a theme switch
//! is a hard cut — still never a blank.
use crate::postfx;

pub struct FadePass {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
    /// The snapshot holds a real frame (at the current size) — set by
    /// `capture`, cleared by `resize`.
    valid: bool,
    /// Whether the surface supports `COPY_SRC` at all.
    enabled: bool,
}

impl FadePass {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        enabled: bool,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fade"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fade.wgsl").into()),
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fade_uniform"),
            size: 16, // 4 × f32
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = postfx::layout(device, "fade_bgl", false);
        let pipeline = blended_pipeline(device, &bgl, &shader, format);
        let sampler = postfx::sampler(device, "fade_sampler");
        let (texture, bind_group) =
            snapshot(device, format, width, height, &bgl, &sampler, &uniform);
        Self {
            pipeline,
            bgl,
            sampler,
            uniform,
            texture,
            bind_group,
            width: width.max(1),
            height: height.max(1),
            valid: false,
            enabled,
        }
    }

    /// Track the surface size; a resize drops the held frame (a mid-fade
    /// resize hard-cuts rather than stretching a stale snapshot).
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (width, height) = (width.max(1), height.max(1));
        if (self.width, self.height) == (width, height) {
            return;
        }
        let (texture, bind_group) = snapshot(
            device,
            self.texture.format(),
            width,
            height,
            &self.bgl,
            &self.sampler,
            &self.uniform,
        );
        self.texture = texture;
        self.bind_group = bind_group;
        self.width = width;
        self.height = height;
        self.valid = false;
    }

    /// Whether `draw` has a frame to show.
    pub fn ready(&self) -> bool {
        self.valid
    }

    /// Copy the just-rendered frame into the snapshot (encoded after all
    /// scene passes, before present). `src` must be surface-sized.
    pub fn capture(&mut self, enc: &mut wgpu::CommandEncoder, src: &wgpu::Texture) {
        if !self.enabled || src.width() != self.width || src.height() != self.height {
            return;
        }
        enc.copy_texture_to_texture(
            src.as_image_copy(),
            self.texture.as_image_copy(),
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.valid = true;
    }

    /// Draw the held frame over `view` at `fade` opacity (1 → all old frame).
    pub fn draw(
        &self,
        enc: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        fade: f32,
    ) {
        if !self.valid {
            return;
        }
        queue.write_buffer(
            &self.uniform,
            0,
            postfx::f32s_as_bytes(&[fade.clamp(0.0, 1.0), 0.0, 0.0, 0.0]),
        );
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fade"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The snapshot texture + its bind group, rebuilt together on resize.
fn snapshot(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    bgl: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
) -> (wgpu::Texture, wgpu::BindGroup) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fade_snapshot"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let bind_group = postfx::bind_group(device, "fade_bg", bgl, &[&view], sampler, uniform);
    (texture, bind_group)
}

/// Like [`postfx::pipeline`] but alpha-blended: the fade composites over the
/// finished frame instead of owning its whole target.
fn blended_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("fade"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fade"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
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
    })
}
