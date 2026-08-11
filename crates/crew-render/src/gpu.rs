use std::sync::Arc;

use winit::window::Window;

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,
    /// Whether the surface supports `COPY_SRC` — the theme-crossfade snapshot
    /// copies the presented frame; without it the fade degrades to a hard cut.
    pub surface_copy: bool,
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
        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window.clone())?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        let caps = surface.get_capabilities(&adapter);
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
        surface.configure(&device, &config);

        Ok(Self {
            device,
            queue,
            surface,
            config,
            format,
            surface_copy,
        })
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
    }
}

#[cfg(test)]
mod tests {
    use wgpu::TextureFormat as F;

    use super::pick_surface_format;

    #[test]
    fn prefers_a_non_srgb_format_for_gamma_space_blending() {
        // Whatever order the platform lists them, non-sRGB wins.
        assert_eq!(
            pick_surface_format(&[F::Bgra8UnormSrgb, F::Bgra8Unorm]),
            F::Bgra8Unorm
        );
        assert_eq!(
            pick_surface_format(&[F::Bgra8Unorm, F::Bgra8UnormSrgb]),
            F::Bgra8Unorm
        );
    }

    #[test]
    fn falls_back_to_the_first_format_when_all_are_srgb() {
        assert_eq!(
            pick_surface_format(&[F::Bgra8UnormSrgb, F::Rgba8UnormSrgb]),
            F::Bgra8UnormSrgb
        );
    }

    mod alpha {
        use wgpu::CompositeAlphaMode as M;

        use super::super::pick_alpha_mode;

        /// Our shaders write straight alpha, so PostMultiplied is the mode that
        /// composites a translucent window correctly.
        #[test]
        fn prefers_post_multiplied() {
            assert_eq!(
                pick_alpha_mode(&[M::Opaque, M::PostMultiplied]),
                M::PostMultiplied
            );
            assert_eq!(
                pick_alpha_mode(&[M::PostMultiplied, M::PreMultiplied, M::Opaque]),
                M::PostMultiplied
            );
        }

        #[test]
        fn falls_back_to_premultiplied_then_to_whatever_exists() {
            assert_eq!(
                pick_alpha_mode(&[M::Opaque, M::PreMultiplied]),
                M::PreMultiplied
            );
            // An Opaque-only platform still has to produce a working surface —
            // the window simply cannot go translucent there.
            assert_eq!(pick_alpha_mode(&[M::Opaque]), M::Opaque);
            assert_eq!(pick_alpha_mode(&[]), M::Auto);
        }
    }
}
