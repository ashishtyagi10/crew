//! Off-screen GPU render harness shared by the chart and sidebar shots, so a
//! widget can be *looked at* rather than only asserted on.
//!
//! Callers pick the canvas size: a chart wants a wide card, the left nav wants
//! a tall narrow column the shape the app actually docks. Everything else —
//! the paper background, the grain, the card the content is laid into — is the
//! same path `build_frame` draws a real frame with.
use crew_render::{CellGrid, CellView, Paint, PaneScene, PaperBgPass};

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
const BPP: u32 = 4;

fn row_padded(w: u32) -> u32 {
    (w * BPP).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
}

/// Render one card filling a `w`×`h` shot: `content` returns the interior's
/// cells and paint at the `(cols, rows, aspect)` it is given, exactly as a
/// sidebar section does. Returns RGBA pixels, or `None` where there is no GPU.
pub fn render_at(
    w: u32,
    h: u32,
    font_px: f32,
    legend: &str,
    content: impl FnOnce(u16, u16, f32) -> (Vec<CellView>, Vec<Paint>),
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
        label: Some("shotgpu"),
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
    let rect = crate::layout::Rect {
        x: 12.0,
        y: 12.0,
        w: w as f32 - 24.0,
        h: h as f32 - 24.0,
    };
    let mut scenes: Vec<PaneScene> = Vec::new();
    crate::panelcard::push_card_art(
        &mut scenes,
        rect,
        cw,
        ch,
        legend,
        crew_theme::theme().legend_off,
        |cols, rows| content(cols, rows, ch / cw),
    );
    grid.set_scene(&device, &scenes);
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
            label: Some("shotgpu_pass"),
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

/// Render and write `<name>.png` under `$CREW_SHOT_DIR`, returning the pixels.
pub fn shot_at(
    name: &str,
    w: u32,
    h: u32,
    font_px: f32,
    legend: &str,
    content: impl FnOnce(u16, u16, f32) -> (Vec<CellView>, Vec<Paint>),
) -> Option<Vec<u8>> {
    let px = render_at(w, h, font_px, legend, content)?;
    let out_dir = std::env::var("CREW_SHOT_DIR").unwrap_or_else(|_| "target/screenshots".into());
    std::fs::create_dir_all(&out_dir).unwrap();
    let path = format!("{out_dir}/{name}.png");
    image::save_buffer(&path, &px, w, h, image::ColorType::Rgba8).unwrap();
    println!("wrote {path}");
    Some(px)
}

/// Count pixels that differ from the page background by more than the grain —
/// "did this widget put ink on the page at all".
pub fn ink(px: &[u8]) -> usize {
    let bg = crew_theme::theme().page_bg;
    px.chunks_exact(4)
        .filter(|p| {
            (p[0] as i32 - bg.0 as i32).abs()
                + (p[1] as i32 - bg.1 as i32).abs()
                + (p[2] as i32 - bg.2 as i32).abs()
                > 40
        })
        .count()
}
