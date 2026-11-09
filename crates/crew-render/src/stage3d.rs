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

/// The key light the app hands over, packed with bit 24 set, or 0 for
/// none (the theme's own accent).
static KEY: AtomicU32 = AtomicU32::new(0);

/// Hand over the accent the app draws with, to light the scene in. Without
/// it the knot took the theme's `accent_default`, so a user who set an
/// accent got a ring in one colour and a knot lit in another.
pub fn set_key(rgb: Option<(u8, u8, u8)>) {
    let packed = rgb.map_or(0, |(r, g, b)| {
        1 << 24 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
    });
    KEY.store(packed, Ordering::Relaxed);
}

pub fn key() -> Option<(u8, u8, u8)> {
    match KEY.load(Ordering::Relaxed) {
        0 => None,
        v => Some(((v >> 16) as u8, (v >> 8) as u8, v as u8)),
    }
}

/// The palette this frame's scene is lit in: the live theme's, keyed by the
/// app's accent. `prepare` rebuilds the scene when it changes, so a new
/// accent relights it on the next frame.
pub(crate) fn wanted_palette() -> Palette {
    let t = crew_theme::theme();
    Palette::lit(t, key().unwrap_or(t.accent_default))
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

/// A scene being drawn: the scene, the texture it draws into, and what the
/// texture last showed.
struct Live {
    shown: Shown,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    /// The texture's size: the window's, rounded up to [`SIZE_STEP`].
    size: (u32, u32),
    /// The phase (bits) and window size the texture was last drawn at, or
    /// `None` when it was never drawn: a frame drawn for a keystroke or a
    /// hover, with the clock and the window unchanged, has nothing new to
    /// show, so it lays down the picture it has.
    drawn: Option<(u32, (u32, u32))>,
}

/// The texture (and threers' renderer, whose depth buffer matches it) grows
/// and shrinks in steps of this many pixels, not with every pixel of a live
/// resize: a drag would otherwise make a texture and a bind group, and resize
/// the renderer, on every frame of it. The blit stretches the texture over
/// the window, and the camera takes the window's own aspect, so the picture
/// is never distorted, only resampled by under one step.
const SIZE_STEP: u32 = 32;

fn stepped((w, h): (u32, u32)) -> (u32, u32) {
    (
        w.div_ceil(SIZE_STEP) * SIZE_STEP,
        h.div_ceil(SIZE_STEP) * SIZE_STEP,
    )
}

/// How often the pass did each costly thing, for the tests.
#[cfg(test)]
#[derive(Default, Debug, PartialEq)]
pub(crate) struct Counts {
    /// threers renderers built.
    pub builds: u32,
    /// Scenes built.
    pub scenes: u32,
    /// Textures made.
    pub targets: u32,
    /// Scenes drawn into their texture.
    pub renders: u32,
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
    #[cfg(test)]
    counts: Counts,
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
            counts: Counts::default(),
        }
    }

    /// Draw this frame's scene into the texture (threers submits its own
    /// work, ahead of the frame's), or let the scene go when the stage is
    /// off. Only a new stage or palette (theme or accent) builds the scene
    /// again, only a size past a [`SIZE_STEP`] makes a new texture, only the
    /// first scene ever builds the renderer, and the scene is drawn only when
    /// its clock or the window's size moved: most frames are a keystroke's,
    /// and those cost the backdrop nothing.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, size: (u32, u32)) {
        let Some(want) = stage() else {
            self.live = None;
            self.on = false;
            return;
        };
        let (w, h) = (size.0.max(1), size.1.max(1));
        let tex = stepped((w, h));
        let palette = wanted_palette();
        let aspect = w as f32 / h as f32;
        if self.renderer.is_none() {
            #[cfg(test)]
            {
                self.counts.builds += 1;
            }
            let r = threers::Renderer::new(
                Arc::new(device.clone()),
                Arc::new(queue.clone()),
                self.format,
                tex.0,
                tex.1,
            );
            self.renderer = Some((r, tex));
        }
        let stale = self
            .live
            .as_ref()
            .is_none_or(|l| l.shown.stage() != want || l.shown.palette() != palette);
        if stale {
            #[cfg(test)]
            {
                self.counts.scenes += 1;
            }
            let shown = Shown::build(want, palette, aspect);
            self.blit.set_strength(queue, strength(want, palette.dark));
            match self.live.as_mut() {
                Some(l) => (l.shown, l.drawn) = (shown, None),
                None => self.live = Some(self.live_at(device, shown, tex)),
            }
        }
        if self.live.as_ref().is_some_and(|l| l.size != tex) {
            let shown = self.live.take().expect("live").shown;
            self.live = Some(self.live_at(device, shown, tex));
        }
        let (Some(l), Some((renderer, at))) = (self.live.as_mut(), self.renderer.as_mut()) else {
            return;
        };
        self.on = true;
        let now = (phase().to_bits(), (w, h));
        if l.drawn == Some(now) {
            return;
        }
        if *at != tex {
            renderer.resize(tex.0, tex.1);
            *at = tex;
        }
        #[cfg(test)]
        {
            self.counts.renders += 1;
        }
        l.shown.render(renderer, &l.view, aspect);
        l.drawn = Some(now);
    }

    /// A texture of `size` for `shown`, not yet drawn.
    fn live_at(&mut self, device: &wgpu::Device, shown: Shown, size: (u32, u32)) -> Live {
        #[cfg(test)]
        {
            self.counts.targets += 1;
        }
        let view = self.target(device, size);
        let bind_group = self.blit.bind(device, &view);
        Live {
            shown,
            view,
            bind_group,
            size,
            drawn: None,
        }
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
