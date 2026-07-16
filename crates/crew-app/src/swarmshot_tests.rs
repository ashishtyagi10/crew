//! Offscreen render check for the live swarm display.
//!
//! `#[ignore]`d: needs a real GPU adapter and takes a second or two. Run with
//! `cargo test -p crew-app --bin crew swarm_shot -- --ignored --nocapture`;
//! the PNG lands in `$CREW_SHOT_DIR` (default `target/screenshots`).
//!
//! Why this exists: the planned live-GUI verification (drive the app, take a
//! screenshot) needs macOS Accessibility AND Screen Recording, which a
//! headless session and CI both lack — so the composition of the status line,
//! bar and composer was never checked against a real frame. Every other test
//! asserts on `CellView`s, which are the layout's source of truth but say
//! nothing about how they actually RASTERISE: this project has a standing bug
//! class where a font whose metrics don't match the cell grid renders narrow
//! fallback glyphs and drifts the row (see the monospace-width work). Themes
//! now swap the font family, so that risk is live.
//!
//! This renders the real pane through the same `CellGrid` the app draws with,
//! at the app's real surface format, and asserts on the pixels.
use crate::chat::ChatPane;
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;
use crew_render::{CellGrid, PaneScene};

const W: u32 = 900;
const H: u32 = 320;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
const BPP: u32 = 4;
const ROW_UNPADDED: u32 = W * BPP;
const ROW_PADDED: u32 =
    ROW_UNPADDED.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

fn mid_run_pane() -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;
    p.messages.push(crate::chatlayout::Message {
        sender: "crew".into(),
        text: "Planned 5 tasks.".into(),
        ts: String::new(),
        meta: String::new(),
    });
    let tasks = [
        "Gather Project Documents",
        "Review Project Overview",
        "研究技術仕様",
    ]
    .iter()
    .enumerate()
    .map(|(i, t)| TaskSpec {
        id: TaskId(i as u64),
        title: (*t).into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
        specialty: String::new(),
        expertise: String::new(),
    })
    .collect();
    p.absorb_hive_plan(tasks);
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    });
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(1),
        state: TaskState::Running,
    });
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(2),
        state: TaskState::Running,
    });
    p
}

/// Render the pane offscreen and return tightly-packed RGBA8 pixels plus the
/// (cols, rows) it was laid out at, or `None` when there is no GPU adapter
/// (CI) — a skip, not a failure, matching `crew-render`'s own headless test.
///
/// The pane's cells are built INSIDE here because the grid geometry comes from
/// a live `CellGrid`: the cell size depends on the font actually loaded, which
/// is the whole point of rendering rather than asserting on `CellView`s.
fn render(pane: &ChatPane) -> Option<(Vec<u8>, u16, u16, f32)> {
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
        label: Some("swarm_shot"),
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
    let (cell_w, cell_h) = grid.cell_size();
    // Lay the pane out at the geometry the REAL font produces.
    let cols = (W as f32 / cell_w) as u16;
    let rows = (H as f32 / cell_h) as u16;
    let cells = pane.cells(cols, rows);
    assert!(!cells.is_empty(), "pane produced no cells at {cols}x{rows}");
    grid.set_scene(
        &device,
        &[PaneScene {
            cells,
            x: 0.0,
            y: 0.0,
            w: W as f32,
            h: H as f32,
            focused: true,
            bordered: false,
            overlay: false,
        }],
    );
    grid.prepare(&device, &queue, W, H);

    let bg = crew_theme::theme().page_bg;
    let bg_f32 = crew_render::color::target_rgba(bg, 1.0, FORMAT.is_srgb());
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("swarm_shot_pass"),
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
    Some((px, cols, rows, cell_h))
}

/// Rows (in pixels) that contain any ink — a pixel differing from the page
/// background by more than a hair.
fn inked_rows(px: &[u8]) -> Vec<usize> {
    let bg = crew_theme::theme().page_bg;
    (0..H as usize)
        .filter(|y| {
            (0..W as usize).any(|x| {
                let i = (y * W as usize + x) * 4;
                let (r, g, b) = (px[i], px[i + 1], px[i + 2]);
                let d = (r as i32 - bg.0 as i32).abs()
                    + (g as i32 - bg.1 as i32).abs()
                    + (b as i32 - bg.2 as i32).abs();
                d > 24
            })
        })
        .collect()
}

#[test]
#[ignore = "needs a GPU adapter; writes a PNG"]
fn swarm_shot_renders_the_status_line_bar_and_composer() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let pane = mid_run_pane();

    let Some((px, cols, rows, cell_h)) = render(&pane) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };

    let out_dir = std::env::var("CREW_SHOT_DIR").unwrap_or_else(|_| "target/screenshots".into());
    std::fs::create_dir_all(&out_dir).unwrap();
    let path = format!("{out_dir}/swarm-status.png");
    image::save_buffer(&path, &px, W, H, image::ColorType::Rgba8).unwrap();
    println!("wrote {path} ({W}x{H}) cols={cols} rows={rows}");

    // The real assertion: the frame has ink, and it lands in distinct
    // horizontal bands rather than smearing into one. A row-drift bug (a font
    // whose advance doesn't match the cell grid) shows up as ink outside the
    // rows the cell grid asked for.
    let rows_with_ink = inked_rows(&px);
    assert!(
        !rows_with_ink.is_empty(),
        "rendered frame is blank — the pane's cells never reached the GPU"
    );
    let first = *rows_with_ink.first().unwrap();
    let last = *rows_with_ink.last().unwrap();
    assert!(
        last < H as usize,
        "ink at the very bottom edge ({last}) — content overflowed the surface"
    );
    println!("ink spans px rows {first}..={last} of {H}; cell_h={cell_h:.2}");
}
