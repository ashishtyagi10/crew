//! The GPU, split into the part there is one of and the part there is one of
//! *per window*.
//!
//! A second crew window used to cost a second everything: its own wgpu
//! instance, its own adapter, its own device and its own queue — a whole
//! parallel driver context for the same card, with no way for anything in one
//! window to be shared with the other, because resources belong to the device
//! that made them.
//!
//! What is genuinely per-window is the **surface**: the swapchain the
//! compositor hands you, its configuration, its pixel format and its size.
//! Everything upstream of that is per-process. So the instance, the adapter,
//! the device and the queue are built once, on the first window, and every
//! window after it gets a surface and nothing else.
//!
//! The adapter is what makes the order awkward: choosing one wants a surface
//! to be compatible with, and a surface wants an instance. So the instance is
//! made first and alone, the first window's surface next, and the adapter and
//! device from that — after which they are kept, and the next window's surface
//! is measured against the adapter that already exists.
use std::sync::{Arc, OnceLock};

use winit::window::Window;

/// The part of the GPU there is one of per process.
pub struct GpuShared {
    /// Kept because every surface is created from it and must not outlive it.
    #[allow(dead_code)]
    instance: &'static wgpu::Instance,
    /// Kept because each new window's surface is measured against it
    /// (`get_capabilities`).
    adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// One window's swapchain.
pub struct Gpu {
    shared: Arc<GpuShared>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,
    /// Whether the surface supports `COPY_SRC` — the theme-crossfade snapshot
    /// copies the presented frame; without it the fade degrades to a hard cut.
    pub surface_copy: bool,
}

/// The one instance. `'static` because a surface borrows from it for as long
/// as it exists, and surfaces outlive any scope this could otherwise sit in.
fn instance() -> &'static wgpu::Instance {
    static INSTANCE: OnceLock<wgpu::Instance> = OnceLock::new();
    INSTANCE.get_or_init(wgpu::Instance::default)
}

/// The adapter, device and queue — chosen once, on the first surface, and
/// handed to every window after it.
///
/// `surface` is what the adapter choice is measured against, and it is an
/// option so the memoization can be tested without a window — there is no
/// headless way to make a surface, and "the second window gets the first
/// window's device" is the whole point of this file.
fn shared_for(surface: Option<&wgpu::Surface<'static>>) -> anyhow::Result<Arc<GpuShared>> {
    static SHARED: OnceLock<Arc<GpuShared>> = OnceLock::new();
    if let Some(s) = SHARED.get() {
        return Ok(Arc::clone(s));
    }
    let adapter = pollster::block_on(instance().request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: surface,
        force_fallback_adapter: false,
    }))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let shared = Arc::new(GpuShared {
        instance: instance(),
        adapter,
        device,
        queue,
    });
    // `get_or_init` rather than `set`: two windows opening in the same instant
    // would both have built one, and the loser's is simply dropped.
    Ok(Arc::clone(SHARED.get_or_init(|| shared)))
}

/// Prefer a NON-sRGB surface so alpha blending happens on gamma-encoded
/// values — the web/CoreText look; glyph antialiasing reads heavier and
/// smoother (glyphon's `ColorMode::Web` documents exactly this target).
/// Colours are still fed via `color::target_rgba`, keyed off the format, so
/// flat theme colours stay byte-exact either way. Falls back to whatever the
/// platform offers when everything is sRGB.
pub(crate) fn pick_surface_format(formats: &[wgpu::TextureFormat]) -> wgpu::TextureFormat {
    formats
        .iter()
        .copied()
        .find(|f| !f.is_srgb())
        .unwrap_or(formats[0])
}

/// Prefer an alpha mode the compositor actually honours, so the window can be
/// made translucent at runtime (Settings → WINDOW → Opacity %). Our shaders write STRAIGHT
/// (non-premultiplied) alpha, hence `PostMultiplied` first; `PreMultiplied` is
/// accepted as a fallback because on the platforms that offer only it, an
/// opaque frame (alpha 1.0 — the default) is identical either way. `Opaque`
/// discards alpha entirely and is the last resort.
pub(crate) fn pick_alpha_mode(modes: &[wgpu::CompositeAlphaMode]) -> wgpu::CompositeAlphaMode {
    use wgpu::CompositeAlphaMode as M;
    for want in [M::PostMultiplied, M::PreMultiplied] {
        if modes.contains(&want) {
            return want;
        }
    }
    modes.first().copied().unwrap_or(M::Auto)
}

impl Gpu {
    pub fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let surface = instance().create_surface(window.clone())?;
        let shared = shared_for(Some(&surface))?;

        // Per SURFACE, not per process: two windows can be on displays that
        // offer different formats and different alpha modes, and each one's
        // pipelines are built for its own.
        let caps = surface.get_capabilities(&shared.adapter);
        let format = pick_surface_format(&caps.formats);
        let surface_copy = caps.usages.contains(wgpu::TextureUsages::COPY_SRC);
        let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        if surface_copy {
            usage |= wgpu::TextureUsages::COPY_SRC;
        }
        let config = wgpu::SurfaceConfiguration {
            usage,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: pick_alpha_mode(&caps.alpha_modes),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&shared.device, &config);

        Ok(Self {
            shared,
            surface,
            config,
            format,
            surface_copy,
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.shared.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.shared.queue
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(self.device(), &self.config);
    }
}

#[cfg(test)]
#[path = "gpu_tests.rs"]
mod tests;
