use std::sync::Arc;

use winit::window::Window;

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,
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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
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
}
