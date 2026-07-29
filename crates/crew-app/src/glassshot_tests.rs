//! Offscreen render check for the frosted-glass pane cards.
//!
//! `#[ignore]`d: needs a real GPU adapter and writes PNGs. Run with
//! `cargo test -p crew-app --bin crew glass_shot -- --ignored --nocapture`;
//! the PNGs land in `$CREW_SHOT_DIR` (default `target/screenshots`).
//!
//! Same reason as [`crate::swarmshot_tests`]: driving the live GUI needs macOS
//! Accessibility AND Screen Recording, which headless sessions and CI lack. The
//! glass unit tests assert on style values and the headless shader test asserts
//! on a synthetic card, but neither shows the sheet in the composition it
//! actually ships in — over the paper grain, under a real border, behind real
//! text. This renders that full stack through the same `CellGrid` and
//! `PaperBgPass` the app draws with, once per theme family.
use crew_render::{CellGrid, PaneScene, PaperBgPass};

const W: u32 = 720;
const H: u32 = 300;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
const BPP: u32 = 4;
const ROW_UNPADDED: u32 = W * BPP;
const ROW_PADDED: u32 =
    ROW_UNPADDED.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// Two panes side by side — one focused, one not — built by the SAME code the
/// app builds frames with.
///
/// This used to hand-roll its `PaneScene`s with `bordered: true`, and that is
/// precisely why the "every pane sits on glass" release shipped drawing no
/// sheet at all: the app's own scenes are all `bordered: false`, so the harness
/// was rendering an arrangement production never produces. A pixel test whose
/// input the app can't generate proves nothing — go through `build_scenes`.
fn panes(cell_w: f32, cell_h: f32) -> Vec<PaneScene> {
    let pane_w = 320.0;
    let pane_h = 200.0;
    let pane_at = |x: f32| crate::pane::Pane {
        content: crate::pane::PaneContent::Far(crate::farpane::FarPane::new(std::env::temp_dir())),
        grid: crew_term::GridSize {
            cols: ((pane_w - 2.0 * cell_w) / cell_w) as u16,
            rows: ((pane_h - 2.0 * cell_h) / cell_h) as u16,
        },
        rect: crate::layout::Rect {
            x,
            y: 50.0,
            w: pane_w,
            h: pane_h,
        },
        label: None,
        name: Some("far".into()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    };
    crate::paneview::build_scenes(
        &[pane_at(30.0), pane_at(370.0)],
        Some(0),
        false,
        None,
        None,
        1.0,
        cell_w,
        cell_h,
    )
}

/// Render one full frame: clear → paper grain → glass → cells → borders → text.
fn render(glass: crew_theme::GlassLevel, opacity: f32) -> Option<Vec<u8>> {
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
        label: Some("glass_shot"),
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
    grid.set_glass(glass);
    let (cell_w, cell_h) = grid.cell_size();
    grid.set_scene(&device, &panes(cell_w, cell_h));
    grid.prepare(&device, &queue, W, H);

    let paper = PaperBgPass::new(&device, FORMAT);
    let bg = crew_theme::theme().page_bg;
    let bg_f32 = crew_render::color::target_rgba(bg, opacity, FORMAT.is_srgb());
    paper.update_uniform(
        &queue,
        bg_f32,
        W as f32,
        H as f32,
        1.0,
        1.3 * crew_theme::theme().grain,
    );

    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("glass_shot_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: bg_f32[0] as f64,
                        g: bg_f32[1] as f64,
                        b: bg_f32[2] as f64,
                        a: bg_f32[3] as f64,
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
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .ok()?;
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .ok()?;
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

/// Mean luminance of a block, for comparing on-card vs off-card regions.
fn mean_lum(px: &[u8], x0: usize, y0: usize, w: usize, h: usize) -> f64 {
    let mut n = 0.0;
    let mut s = 0.0;
    for y in y0..(y0 + h) {
        for x in x0..(x0 + w) {
            let i = (y * W as usize + x) * 4;
            s += 0.2126 * px[i] as f64 + 0.7152 * px[i + 1] as f64 + 0.0722 * px[i + 2] as f64;
            n += 1.0;
        }
    }
    s / n
}

fn shot(name: &str, id: crew_theme::ThemeId, glass: crew_theme::GlassLevel, opacity: f32) {
    crew_theme::set_theme(id);
    let Some(px) = render(glass, opacity) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let out_dir = std::env::var("CREW_SHOT_DIR").unwrap_or_else(|_| "target/screenshots".into());
    std::fs::create_dir_all(&out_dir).unwrap();
    let path = format!("{out_dir}/glass-{name}.png");
    image::save_buffer(&path, &px, W, H, image::ColorType::Rgba8).unwrap();

    // Compare the SAME scene rendered with the sheet off, sampled inside the
    // left pane. The old assertion was on-card vs the gap between panes, which
    // a pane's own cell backgrounds satisfy whether or not any glass is drawn —
    // it passed at full strength with the sheet disabled entirely. The only
    // honest question is whether the glass level moves the pixels.
    let flat = render(crew_theme::GlassLevel::Off, opacity).expect("adapter was available above");
    let on = mean_lum(&px, 60, 120, 200, 60);
    let bare = mean_lum(&flat, 60, 120, 200, 60);
    println!(
        "wrote {path}  on_card={on:.1} glass_off={bare:.1} delta={:.1}",
        on - bare
    );
    assert!(
        (on - bare).abs() > 1.5,
        "{name}: the sheet changes nothing — glass {on:.1} vs off {bare:.1}"
    );
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn glass_shot_every_theme_family() {
    let _g = crate::app::theme_test_guard();
    use crew_theme::{GlassLevel as G, ThemeId as T};
    shot("light", T::PaperLight, G::Medium, 1.0);
    shot("dark", T::PaperDark, G::Medium, 1.0);
    shot("crt", T::CrtGreen, G::Medium, 1.0);
    shot("light-high", T::PaperLight, G::High, 1.0);
    shot("dark-high", T::PaperDark, G::High, 1.0);
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn glass_shot_translucent_window() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let Some(px) = render(crew_theme::GlassLevel::Medium, 0.6) else {
        eprintln!("no GPU adapter — skipping");
        return;
    };
    let out_dir = std::env::var("CREW_SHOT_DIR").unwrap_or_else(|_| "target/screenshots".into());
    std::fs::create_dir_all(&out_dir).unwrap();
    let path = format!("{out_dir}/glass-window-60.png");
    image::save_buffer(&path, &px, W, H, image::ColorType::Rgba8).unwrap();

    // The bare page carries the window opacity, while glyph coverage pushes
    // alpha back up — so text stays readable against whatever is behind.
    let page_a = px[(10 * W as usize + 10) * 4 + 3];
    println!("wrote {path}  page_alpha={page_a}");
    assert!(
        (140..=175).contains(&page_a),
        "page alpha {page_a} should track the 0.6 window opacity (~153)"
    );
}
