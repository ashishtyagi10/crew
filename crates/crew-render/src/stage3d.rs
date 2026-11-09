//! The 3D backdrop: a threers scene (`stage3dscene`) drawn on crew's own
//! device into a texture of the window's size, then laid down first in the
//! scene pass, over the page's clear and under the panes' glass, the way
//! neon's grid is (`holo`).
//!
//! Off (the default) it holds no scene and no texture, and the pass draws
//! nothing. Turned on, the renderer is built at the next frame, its
//! pipelines with it (threers 0.0.10 has no lazy build yet), so that one
//! frame is slow. Turning it off lets the scene and its texture go but keeps
//! the renderer: `/theme auto` walks into prism, whose scene is on by
//! default, once a lap, and rebuilding every pipeline each lap would stall
//! a frame every lap rather than once. The scene moves
//! on the clock the app hands
//! over ([`set_phase`]), which turns only while a pane works and holds at
//! Motion off, so an idle window draws no frames for it.
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

use crate::stage3dprism::Prisms;
use crate::stage3dscene::{Built, Palette};

/// The scenes there are.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    /// A metal torus knot with a ring of crystals going round it.
    Orbit,
    /// Faceted glass prisms drifting in a fog of the page: the prism
    /// theme's own scene (`crew_theme::ThemeId::scene`).
    Prism,
}

impl Stage {
    pub const ALL: [Stage; 2] = [Stage::Orbit, Stage::Prism];

    pub fn name(self) -> &'static str {
        match self {
            Stage::Orbit => "orbit",
            Stage::Prism => "prism",
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
/// Prism's low-contrast glass in fog takes a little more than orbit's lit
/// metal: at orbit's strength its prisms read as smudges.
pub(crate) fn strength(stage: Stage, dark: bool) -> f32 {
    match (stage, dark) {
        (_, false) => 0.45,
        (Stage::Prism, true) => 0.8,
        (Stage::Orbit, true) => 0.7,
    }
}

/// One of the scenes, built.
enum Shown {
    Orbit(Built),
    Prism(Prisms),
}

impl Shown {
    fn build(stage: Stage, p: Palette, aspect: f32) -> Self {
        match stage {
            Stage::Orbit => Shown::Orbit(Built::orbit(p, aspect)),
            Stage::Prism => Shown::Prism(Prisms::new(p, aspect)),
        }
    }

    fn stage(&self) -> Stage {
        match self {
            Shown::Orbit(_) => Stage::Orbit,
            Shown::Prism(_) => Stage::Prism,
        }
    }

    fn palette(&self) -> Palette {
        match self {
            Shown::Orbit(b) => b.palette,
            Shown::Prism(b) => b.palette,
        }
    }

    /// Pose it at `phase` for `aspect` and draw it into `view`.
    fn render(&mut self, r: &mut threers::Renderer, view: &wgpu::TextureView, aspect: f32) {
        let phase = phase();
        match self {
            Shown::Orbit(b) => {
                b.set_aspect(aspect);
                b.pose(phase);
                r.render(&mut b.scene, &b.camera, view, false);
            }
            Shown::Prism(b) => {
                b.set_aspect(aspect);
                b.pose(phase);
                r.render(&mut b.scene, &b.camera, view, false);
            }
        }
    }
}

/// A scene being drawn: the scene and the texture it draws into.
struct Live {
    shown: Shown,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    size: (u32, u32),
}

pub struct Stage3dPass {
    blit: crate::stage3dblit::Blit,
    format: wgpu::TextureFormat,
    /// threers' renderer and the size it was last set to. Built on the first
    /// frame any scene shows and kept while the stage is off (see the
    /// module doc): its pipelines are the slow part.
    renderer: Option<(threers::Renderer, (u32, u32))>,
    live: Option<Live>,
    on: bool,
    /// How many times a renderer was built, for the tests.
    #[cfg(test)]
    builds: u32,
}

impl Stage3dPass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            blit: crate::stage3dblit::Blit::new(device, format),
            format,
            renderer: None,
            live: None,
            on: false,
            #[cfg(test)]
            builds: 0,
        }
    }

    /// Draw this frame's scene into the texture (threers submits its own
    /// work, ahead of the frame's), or let the scene go when the stage is
    /// off. A new stage, palette or size builds the scene again; only the
    /// first scene ever builds the renderer.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, size: (u32, u32)) {
        let Some(want) = stage() else {
            self.live = None;
            self.on = false;
            return;
        };
        let (w, h) = (size.0.max(1), size.1.max(1));
        let palette = Palette::of(crew_theme::theme());
        let aspect = w as f32 / h as f32;
        let (renderer, at) = self.renderer.get_or_insert_with(|| {
            #[cfg(test)]
            {
                self.builds += 1;
            }
            let r = threers::Renderer::new(
                Arc::new(device.clone()),
                Arc::new(queue.clone()),
                self.format,
                w,
                h,
            );
            (r, (w, h))
        });
        if *at != (w, h) {
            renderer.resize(w, h);
            *at = (w, h);
        }
        let stale = self.live.as_ref().is_none_or(|l| {
            l.size != (w, h) || l.shown.stage() != want || l.shown.palette() != palette
        });
        if stale {
            let view = self.target(device, (w, h));
            let bind_group = self.blit.bind(device, &view);
            self.live = Some(Live {
                shown: Shown::build(want, palette, aspect),
                view,
                bind_group,
                size: (w, h),
            });
        }
        let (Some(l), Some((renderer, _))) = (self.live.as_mut(), self.renderer.as_mut()) else {
            return;
        };
        l.shown.render(renderer, &l.view, aspect);
        self.blit.set_strength(queue, strength(want, palette.dark));
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
