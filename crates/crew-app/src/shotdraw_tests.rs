//! The GPU plumbing every shot shares: an off-screen target the size of the
//! surface under test, the paper background the real frame draws first, the
//! cell grid on top, and the readback that turns it into pixels you can look
//! at.
//!
//! Split out of `shotgpu_tests` so a widget that draws its OWN card — the
//! input bar, whose fieldset frame *is* the thing being judged — can be shot
//! at the full canvas instead of being nested inside the harness's card.
use crew_render::{CellGrid, PaneScene, PaperBgPass};

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
const BPP: u32 = 4;

fn row_padded(w: u32) -> u32 {
    (w * BPP).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
}

/// Render `scenes` (built from the cell size the grid reports) into a `w`×`h`
/// canvas and hand back RGBA pixels. `None` where there is no GPU adapter.
pub fn draw(
    w: u32,
    h: u32,
    font_px: f32,
    scenes: impl FnOnce(f32, f32) -> Vec<PaneScene>,
) -> Option<Vec<u8>> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;

    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shotdraw"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    let padded = row_padded(w);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut grid = CellGrid::new(&device, &queue, FORMAT, font_px);
    let (cw, ch) = grid.cell_size();
    grid.set_scene(&device, &scenes(cw, ch));
    grid.prepare(&device, &queue, w, h);

    let paper = PaperBgPass::new(&device, FORMAT);
    let bg = crew_theme::theme().page_bg;
    let bg_f32 = crew_render::color::target_rgba(bg, 1.0, FORMAT.is_srgb());
    paper.update_uniform(
        &queue,
        bg_f32,
        (w as f32, h as f32),
        1.0,
        1.3 * crew_theme::theme().grain,
        None,
    );

    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shotdraw_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: bg_f32[0] as f64,
                        g: bg_f32[1] as f64,
                        b: bg_f32[2] as f64,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        paper.draw(&mut pass);
        grid.draw(&mut pass);
    }
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(enc.finish()));
    let wait = || wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    };
    device.poll(wait()).ok()?;
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wait()).ok()?;
    let mapped = readback.slice(..).get_mapped_range().to_vec();
    readback.unmap();

    let unpadded = (w * BPP) as usize;
    let mut px = Vec::with_capacity((w * h * BPP) as usize);
    for row in 0..h as usize {
        let src = row * padded as usize;
        px.extend_from_slice(&mapped[src..src + unpadded]);
    }
    for c in px.chunks_exact_mut(4) {
        c.swap(0, 2); // BGRA -> RGBA
    }
    Some(px)
}

/// Write `<name>.png` under `$CREW_SHOT_DIR` (default `target/screenshots`).
pub fn write_png(name: &str, px: &[u8], w: u32, h: u32) {
    let out_dir = std::env::var("CREW_SHOT_DIR").unwrap_or_else(|_| "target/screenshots".into());
    std::fs::create_dir_all(&out_dir).unwrap();
    let path = format!("{out_dir}/{name}.png");
    image::save_buffer(&path, px, w, h, image::ColorType::Rgba8).unwrap();
    println!("wrote {path}");
}
