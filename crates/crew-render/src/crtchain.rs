//! The whole CRT post-process, owned as one unit: the off-screen scene
//! target, the half-res bloom chain, and the final composite pass. The
//! renderer's frame path and the headless GPU test both drive THIS type, so
//! what the test measures is the real composite — bloom included — not a
//! test-only re-plumbing of the passes.
use crate::bloom::Bloom;
use crate::crt::CrtPass;
use crate::scenetarget::SceneTarget;
use crew_theme::CrtStyle;

/// Whether the frame must leave the composite PREMULTIPLIED at `window_opacity`.
///
/// The window server composites crew premultiplied — shown = rgb + (1 − a) ·
/// desktop — and every pass writes straight colour. On a dark page that is
/// near enough (dark colour is near zero either way), and the tubes' glow is
/// tuned to add light over the desktop. A LIGHT page is not: its colour is
/// added to the desktop's whole, so a see-through glass window washed out to
/// white instead of showing what is behind it. Liquid glass is see-through by
/// design (`LiquidStyle::window`), so it leaves premultiplied whenever the
/// window is sheer, and the chain runs for it even with no tube of its own.
pub fn premultiplies(window_opacity: f32) -> bool {
    window_opacity < 1.0 && crew_theme::theme().liquid.is_some()
}

/// The composite as a plain copy: what the chain draws when it runs only to
/// premultiply ([`premultiplies`]) under a theme with no tube of its own.
const CLEAR: CrtStyle = CrtStyle {
    scanline: 0.0,
    glow: 0.0,
    glow_radius: CrtStyle::DEFAULT.glow_radius,
    flicker: 0.0,
    core: 0.0,
    etch: 0.0,
    shade: 0.0,
};

pub struct CrtChain {
    pass: CrtPass,
    bloom: Bloom,
    target: SceneTarget,
    format: wgpu::TextureFormat,
    style: Option<CrtStyle>,
    time: f32,
    flicker: f32,
    /// Leave the composite premultiplied (see [`premultiplies`]).
    premul: bool,
}

impl CrtChain {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, w: u32, h: u32) -> Self {
        let mut chain = Self {
            pass: CrtPass::new(device, format),
            bloom: Bloom::new(device),
            target: SceneTarget::new(device, format, w, h),
            format,
            style: None,
            time: 0.0,
            flicker: 0.0,
            premul: false,
        };
        chain.bind(device, w, h);
        chain
    }

    /// Wire bloom to the scene target and the composite to both outputs.
    fn bind(&mut self, device: &wgpu::Device, w: u32, h: u32) {
        self.bloom.set_source(device, &self.target.view, w, h);
        // `bind` always runs right after `bloom.set_source`, so output exists.
        let bloom_view = self.bloom.output().expect("bloom targets just built");
        self.pass.set_source(device, &self.target.view, bloom_view);
    }

    /// Track the surface size: recreate the scene target and bloom targets
    /// (and rebind everything) only when the size actually changed.
    pub fn resize(&mut self, device: &wgpu::Device, w: u32, h: u32) {
        if self.target.matches(w, h) {
            return;
        }
        self.target = SceneTarget::new(device, self.format, w, h);
        self.bind(device, w, h);
    }

    /// The off-screen view the scene passes draw into while CRT is on.
    pub fn scene_view(&self) -> &wgpu::TextureView {
        &self.target.view
    }

    /// The scene texture itself — the headless harness writes its source
    /// patterns straight into the real target instead of a stand-in.
    pub fn scene_texture(&self) -> &wgpu::Texture {
        &self.target.texture
    }

    /// The active tube tuning; `None` turns the whole chain off.
    pub fn set_style(&mut self, style: Option<CrtStyle>) {
        self.style = style;
    }

    pub fn style(&self) -> Option<CrtStyle> {
        self.style
    }

    /// Leave the composite premultiplied (see [`premultiplies`]). While set
    /// the chain must run even with no style: it is the frame's last pass.
    pub fn set_premultiply(&mut self, on: bool) {
        self.premul = on;
    }

    /// Whether the frame goes through the chain at all: a tube, or a
    /// premultiply only it can do.
    pub fn active(&self) -> bool {
        self.style.is_some() || self.premul
    }

    /// Per-frame animation inputs: `time` seeds the flicker hash, `flicker`
    /// is its amplitude (0 = a static tube).
    pub fn set_anim(&mut self, time: f32, flicker: f32) {
        self.time = time;
        self.flicker = flicker;
    }

    /// Write the frame's uniforms (composite + bloom) from the active style.
    /// `ink` picks the halo's direction for this frame: `false` is the tube's
    /// added light (dark pages), `true` the coloured shadow a light page needs
    /// (see `bloom.wgsl`). The caller owns the page, so it owns the choice.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, w: f32, h: f32, ink: bool) {
        let mut style = self.style.unwrap_or(CLEAR);
        // On a light page the halo is a coloured SHADOW, not added light:
        // bloom's ink pass hands the composite the blurred complement of
        // whatever is colourful, and SUBTRACTING that tints the page toward
        // the colour instead of blowing it to white. Same knob, opposite
        // direction — the theme still states one positive glow strength, and
        // flipping the sign here keeps the composite one `col += bloom *
        // glow` for both appearances.
        if ink {
            style.glow = -style.glow;
        }
        self.pass
            .update_uniform(queue, w, h, self.time, self.flicker, style, self.premul);
        self.bloom.update_uniform(queue, style.glow_radius, ink);
    }

    /// Encode the full post-process: three bloom passes over the scene, then
    /// the composite onto `surface_view`.
    pub fn encode(&self, enc: &mut wgpu::CommandEncoder, surface_view: &wgpu::TextureView) {
        self.bloom.encode(enc);
        self.pass.encode(enc, surface_view);
    }
}
