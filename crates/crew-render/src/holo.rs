//! The holo grid: neon's backdrop (crew-theme `holo`), a pass of its own
//! drawn first in the scene, over the page's clear and under the panes'
//! glass — so each pane floats over the grid and shows it through.
//!
//! It draws only on a theme with a [`crew_theme::holo::Backdrop`]; on every
//! other theme the pass is skipped and the frame is what it was. Its sparks
//! rise on the clock the app hands over ([`set_phase`]), which turns only
//! while a pane works and holds at Motion off, so an idle window costs no
//! frames and a still one draws its sparks where they stopped.
use std::sync::atomic::{AtomicU32, Ordering};

use wgpu::util::DeviceExt as _;

/// The clock: the app's wash phase, in turns, as `f32` bits.
static PHASE: AtomicU32 = AtomicU32::new(0);

/// Hand over this frame's phase (turns, `0..1`).
pub fn set_phase(turns: f32) {
    PHASE.store(turns.to_bits(), Ordering::Relaxed);
}

fn phase() -> f32 {
    f32::from_bits(PHASE.load(Ordering::Relaxed))
}

/// Floats in the uniform: size + cell, the line, the two sparks, the clock.
const UNIFORM_FLOATS: usize = 20;

fn f32s_as_bytes(data: &[f32]) -> &[u8] {
    // SAFETY: f32 is Pod (no padding, valid for any bit pattern).
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}

/// What the uniform holds for `b` on a `size` px target with `cell` px
/// cells at clock `phase`, colours converted for the target.
fn uniform(
    b: &crew_theme::holo::Backdrop,
    size: (f32, f32),
    cell: (f32, f32),
    phase: f32,
    srgb: bool,
) -> [f32; UNIFORM_FLOATS] {
    let c = |rgb, a| crate::color::target_rgba(rgb, a, srgb);
    let (line, spark, spark2) = (c(b.line, b.strength), c(b.spark, 1.0), c(b.spark2, 1.0));
    let mut u = [0.0; UNIFORM_FLOATS];
    u[..4].copy_from_slice(&[size.0, size.1, cell.0, cell.1]);
    u[4..8].copy_from_slice(&line);
    u[8..12].copy_from_slice(&spark);
    u[12..16].copy_from_slice(&spark2);
    u[16] = phase;
    u
}

/// The backdrop pass.
pub struct HoloLayer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    buf: wgpu::Buffer,
    srgb: bool,
    /// Whether the live theme has a grid: [`Self::draw`] does nothing if not.
    on: bool,
}

impl HoloLayer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("holo"),
            source: wgpu::ShaderSource::Wgsl(include_str!("holo.wgsl").into()),
        });
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("holo_uniform"),
            contents: f32s_as_bytes(&[0.0f32; UNIFORM_FLOATS]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("holo_bgl"),
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
            label: Some("holo_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("holo_layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("holo_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[], // fullscreen triangle
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
        Self {
            pipeline,
            bind_group,
            buf,
            srgb: format.is_srgb(),
            on: false,
        }
    }

    /// Follow the live theme and the clock, on a `size` px target whose text
    /// cells are `cell` px.
    pub fn prepare(&mut self, queue: &wgpu::Queue, size: (f32, f32), cell: (f32, f32)) {
        let b = crew_theme::holo::backdrop(crew_theme::theme());
        self.on = b.is_some();
        if let Some(b) = b {
            let u = uniform(&b, size, cell, phase(), self.srgb);
            queue.write_buffer(&self.buf, 0, f32s_as_bytes(&u));
        }
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if !self.on {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
#[path = "holo_tests.rs"]
mod tests;
