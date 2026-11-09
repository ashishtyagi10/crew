//! The 3D backdrop: a threers scene (`stage3dscene`) drawn on crew's own
//! device into a texture of the window's size, then laid down first in the
//! scene pass, over the page's clear and under the panes' glass, the way
//! neon's grid is (`holo`).
//!
//! Off (the default) it holds nothing: no threers renderer, no texture, and
//! the pass draws nothing. Turned on, the renderer is built at the next
//! frame, its pipelines with it (threers 0.0.10 has no lazy build yet), so
//! that one frame is slow; turning it off lets all of it go. The scene moves
//! on the clock the app hands
//! over ([`set_phase`]), which turns only while a pane works and holds at
//! Motion off, so an idle window draws no frames for it.
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

use crate::stage3dscene::{Built, Palette};

/// The scenes there are. One so far.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    /// A metal torus knot with a ring of crystals going round it.
    Orbit,
}

impl Stage {
    pub const ALL: [Stage; 1] = [Stage::Orbit];

    pub fn name(self) -> &'static str {
        match self {
            Stage::Orbit => "orbit",
        }
    }

    pub fn parse(s: &str) -> Option<Stage> {
        Self::ALL.into_iter().find(|k| k.name() == s)
    }
}

static STAGE: AtomicU8 = AtomicU8::new(0);
static PHASE: AtomicU32 = AtomicU32::new(0);

/// Held by every test that sets the stage: it is one switch for the process.
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Which scene to draw behind the panes, or `None` for none.
pub fn set_stage(s: Option<Stage>) {
    STAGE.store(s.map_or(0, |s| s as u8 + 1), Ordering::Relaxed);
}

pub fn stage() -> Option<Stage> {
    match STAGE.load(Ordering::Relaxed) {
        0 => None,
        n => Stage::ALL.get(usize::from(n - 1)).copied(),
    }
}

/// Hand over this frame's clock (turns).
pub fn set_phase(turns: f32) {
    PHASE.store(turns.to_bits(), Ordering::Relaxed);
}

fn phase() -> f32 {
    f32::from_bits(PHASE.load(Ordering::Relaxed))
}

/// How much of the scene shows: dim enough that a pane's words over its
/// glass still read first. A light page takes less, as it shows more.
pub(crate) fn strength(dark: bool) -> f32 {
    if dark {
        0.7
    } else {
        0.45
    }
}

/// A scene being drawn: threers' renderer, the scene, and the texture it
/// draws into.
struct Live {
    renderer: threers::Renderer,
    built: Built,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    size: (u32, u32),
}

pub struct Stage3dPass {
    blit: crate::stage3dblit::Blit,
    format: wgpu::TextureFormat,
    live: Option<Live>,
    on: bool,
}

impl Stage3dPass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            blit: crate::stage3dblit::Blit::new(device, format),
            format,
            live: None,
            on: false,
        }
    }

    /// Draw this frame's scene into the texture (threers submits its own
    /// work, ahead of the frame's), or let it all go when the stage is off.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, size: (u32, u32)) {
        let Some(Stage::Orbit) = stage() else {
            self.live = None;
            self.on = false;
            return;
        };
        let (w, h) = (size.0.max(1), size.1.max(1));
        let palette = Palette::of(crew_theme::theme());
        let aspect = w as f32 / h as f32;
        let stale = self
            .live
            .as_ref()
            .is_none_or(|l| l.built.palette != palette);
        if self.live.as_ref().is_some_and(|l| l.size != (w, h)) || stale {
            let renderer = match self.live.take() {
                Some(mut l) => {
                    l.renderer.resize(w, h);
                    l.renderer
                }
                None => threers::Renderer::new(
                    Arc::new(device.clone()),
                    Arc::new(queue.clone()),
                    self.format,
                    w,
                    h,
                ),
            };
            let view = self.target(device, (w, h));
            let bind_group = self.blit.bind(device, &view);
            self.live = Some(Live {
                renderer,
                built: Built::orbit(palette, aspect),
                view,
                bind_group,
                size: (w, h),
            });
        }
        let Some(l) = self.live.as_mut() else { return };
        l.built.set_aspect(aspect);
        l.built.pose(phase());
        l.renderer
            .render(&mut l.built.scene, &l.built.camera, &l.view, false);
        self.blit.set_strength(queue, strength(palette.dark));
        self.on = true;
    }

    fn target(&self, device: &wgpu::Device, (w, h): (u32, u32)) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("stage3d"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if let Some(l) = self.live.as_ref().filter(|_| self.on) {
            self.blit.draw(pass, &l.bind_group);
        }
    }
}

#[cfg(test)]
#[path = "stage3d_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "stage3d_gpu_tests.rs"]
mod gpu_tests;
