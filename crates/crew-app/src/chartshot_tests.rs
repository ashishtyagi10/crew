//! Off-screen render of the [`crate::plot`] widgets, so a chart can be *looked
//! at* rather than only asserted on.
//!
//! `#[ignore]`d: needs a real GPU adapter and writes PNGs. Run with
//! `cargo test -p crew-app --bin crew chart_shot -- --ignored --nocapture`;
//! the PNGs land in `$CREW_SHOT_DIR` (default `target/screenshots`).
//!
//! Driving the live GUI needs macOS Accessibility AND Screen Recording, which
//! this session does not have — and a chart is exactly the kind of thing that
//! passes every unit test and still comes out an unreadable smear. Each chart
//! iteration adds its shot here, rendered through the same `CellGrid` the app
//! draws frames with, on a real card over the real paper background.
use crew_render::{CellGrid, CellView, Paint, PaneScene, PaperBgPass};

const W: u32 = 480;
const H: u32 = 260;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
const BPP: u32 = 4;
const ROW_UNPADDED: u32 = W * BPP;
const ROW_PADDED: u32 =
    ROW_UNPADDED.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// Render one card filling the shot: `content` returns the interior's cells
/// and paint at the `(cols, rows)` it is given, exactly as a sidebar section
/// does. Returns the RGBA pixels, or `None` where there is no GPU.
fn render(
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
        label: Some("chart_shot"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
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
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (ROW_PADDED * H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut grid = CellGrid::new(&device, &queue, FORMAT, 13.0);
    let (cw, ch) = grid.cell_size();
    let rect = crate::layout::Rect {
        x: 12.0,
        y: 12.0,
        w: W as f32 - 24.0,
        h: H as f32 - 24.0,
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
    grid.prepare(&device, &queue, W, H);

    let paper = PaperBgPass::new(&device, FORMAT);
    let bg = crew_theme::theme().page_bg;
    let bg_f32 = crew_render::color::target_rgba(bg, 1.0, FORMAT.is_srgb());
    paper.update_uniform(
        &queue,
        bg_f32,
        (W as f32, H as f32),
        1.0,
        1.3 * crew_theme::theme().grain,
        None,
    );

    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("chart_shot_pass"),
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
                bytes_per_row: Some(ROW_PADDED),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
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
    let padded = readback.slice(..).get_mapped_range().to_vec();
    readback.unmap();

    let mut px = Vec::with_capacity((W * H * BPP) as usize);
    for row in 0..H as usize {
        let src = row * ROW_PADDED as usize;
        px.extend_from_slice(&padded[src..src + ROW_UNPADDED as usize]);
    }
    for c in px.chunks_exact_mut(4) {
        c.swap(0, 2); // BGRA -> RGBA
    }
    Some(px)
}

/// Render and write `chart-<name>.png`, returning the pixels for assertions.
fn shot(
    name: &str,
    legend: &str,
    content: impl FnOnce(u16, u16, f32) -> (Vec<CellView>, Vec<Paint>),
) -> Option<Vec<u8>> {
    let px = render(legend, content)?;
    let out_dir = std::env::var("CREW_SHOT_DIR").unwrap_or_else(|_| "target/screenshots".into());
    std::fs::create_dir_all(&out_dir).unwrap();
    let path = format!("{out_dir}/chart-{name}.png");
    image::save_buffer(&path, &px, W, H, image::ColorType::Rgba8).unwrap();
    println!("wrote {path}");
    Some(px)
}

/// A plausible CPU trace: a slow swell with a spike, the shape the sidebar
/// actually shows.
fn series(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let base = 0.28 + 0.22 * (t * 6.0).sin();
            let spike = if (0.62..0.70).contains(&t) { 0.45 } else { 0.0 };
            (base + spike).clamp(0.02, 0.98)
        })
        .collect()
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chart_shot_area() {
    let _g = crate::app::theme_test_guard();
    let px = shot("area", "SYSTEM", |cols, rows, aspect| {
        let mut c = crate::plot::Canvas::new(cols, rows, aspect);
        let (w, h) = c.size();
        crate::plot::area::draw(
            &mut c,
            (0.0, 0.0, w, h),
            &series(48),
            crate::palette::accent(),
        );
        (Vec::new(), c.paint())
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    // The chart put ink on the page: some pixel inside the card differs from
    // the page it is drawn on by more than the paper grain.
    let bg = crew_theme::theme().page_bg;
    let ink = px
        .chunks_exact(4)
        .filter(|p| {
            (p[0] as i32 - bg.0 as i32).abs()
                + (p[1] as i32 - bg.1 as i32).abs()
                + (p[2] as i32 - bg.2 as i32).abs()
                > 40
        })
        .count();
    assert!(ink > 2000, "the chart drew something: {ink} ink pixels");
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chart_shot_crew_donut() {
    let _g = crate::app::theme_test_guard();
    let m = crate::crewpie::Mix {
        working: 3,
        waiting: 1,
        idle: 2,
    };
    let mut pulse = crate::spark::History::new(64);
    for (i, v) in (0..48).map(|i| (i, (i % 7) as u64)) {
        let _ = i;
        pulse.push(v);
    }
    let px = shot("donut", "PANES", |cols, rows, aspect| {
        let _ = rows;
        (
            crate::crewpie::cells(&m, cols, 0),
            crate::crewpie::paint(&m, cols, 0, aspect, &pulse),
        )
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let bg = crew_theme::theme().page_bg;
    let ink = px
        .chunks_exact(4)
        .filter(|p| {
            (p[0] as i32 - bg.0 as i32).abs()
                + (p[1] as i32 - bg.1 as i32).abs()
                + (p[2] as i32 - bg.2 as i32).abs()
                > 40
        })
        .count();
    assert!(ink > 2000, "the donut drew something: {ink} ink pixels");
}
