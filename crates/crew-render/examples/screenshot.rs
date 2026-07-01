//! Headless render-to-PNG harness for paper theme verification.
//!
//! Renders a representative crew frame (paper-bg + panes + sidebar + input bar)
//! offscreen via Metal/wgpu and writes two PNGs — one per theme variant.
//!
//! Run: `cargo run --example screenshot -p crew-render`

use crew_render::{CellGrid, CellView, PaneScene, PaperBgPass};
use crew_theme::ThemeId;

const W: u32 = 1200;
const H: u32 = 800;
// Match the real app's surface format (Metal/wgpu picks Bgra8UnormSrgb on macOS).
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

// 256-byte aligned bytes-per-row for W columns of BGRA8.
const BYTES_PER_PIXEL: u32 = 4;
const ROW_BYTES_UNPADDED: u32 = W * BYTES_PER_PIXEL;
const ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
const ROW_BYTES_PADDED: u32 = ROW_BYTES_UNPADDED.div_ceil(ALIGN) * ALIGN;
const BUF_SIZE: u64 = (ROW_BYTES_PADDED * H) as u64;

fn main() {
    // --- headless wgpu init (same pattern as paperbg_headless.rs test) ---
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("no GPU adapter found — cannot run headless screenshot");

    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    // Create the offscreen texture once; reuse across both theme renders.
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("screenshot_tex"),
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
    let tex_view = tex.create_view(&Default::default());

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: BUF_SIZE,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Build rendering pipeline components. CellGrid owns glyphon + quad/border layers.
    let paper_bg = PaperBgPass::new(&device, FORMAT);
    let mut cell_grid = CellGrid::new(&device, &queue, FORMAT, 13.0);

    let (cell_w, cell_h) = cell_grid.cell_size();

    // render every theme
    for (theme_id, out_path) in [
        (
            ThemeId::PaperLight,
            "/private/tmp/claude-501/-Users-atyagi-code-crew/7f6ecec7-c641-4cc2-a923-a17dea6afba0/scratchpad/crew-paper-light.png",
        ),
        (
            ThemeId::PaperDark,
            "/private/tmp/claude-501/-Users-atyagi-code-crew/7f6ecec7-c641-4cc2-a923-a17dea6afba0/scratchpad/crew-paper-dark.png",
        ),
        (
            ThemeId::CrtGreen,
            "/private/tmp/claude-501/-Users-atyagi-code-crew/7f6ecec7-c641-4cc2-a923-a17dea6afba0/scratchpad/crt-green.png",
        ),
        (
            ThemeId::CrtAmber,
            "/private/tmp/claude-501/-Users-atyagi-code-crew/7f6ecec7-c641-4cc2-a923-a17dea6afba0/scratchpad/crt-amber.png",
        ),
        (
            ThemeId::CrtBlue,
            "/private/tmp/claude-501/-Users-atyagi-code-crew/7f6ecec7-c641-4cc2-a923-a17dea6afba0/scratchpad/crt-blue.png",
        ),
    ] {
        crew_theme::set_theme(theme_id);

        // Build the scene AFTER set_theme: `place_str` bakes `CellView.bg` from the
        // active theme's page_bg, so cells must be constructed per-theme for the
        // CellGrid bg-skip (cell.bg == page_bg) to fire and stay transparent.
        let panes = build_scene(cell_w, cell_h);

        // Upload scene (quads + borders + text buffers).
        cell_grid.set_scene(&device, &panes);
        cell_grid.prepare(&device, &queue, W, H);

        // Encode frame.
        let bg = crew_theme::theme().page_bg;
        let bg_f32 = [
            bg.0 as f32 / 255.0,
            bg.1 as f32 / 255.0,
            bg.2 as f32 / 255.0,
            1.0_f32,
        ];
        paper_bg.update_uniform(&queue, bg_f32, W as f32, H as f32, 1.0, 1.0);

        let mut enc =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("screenshot_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &tex_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg.0 as f64 / 255.0,
                            g: bg.1 as f64 / 255.0,
                            b: bg.2 as f64 / 255.0,
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
            // Pass order mirrors renderer.rs: paperbg → quads → borders → text
            paper_bg.draw(&mut pass);
            cell_grid.draw(&mut pass);
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
                    bytes_per_row: Some(ROW_BYTES_PADDED),
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
            .expect("poll failed");

        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("poll (map) failed");

        let padded = readback.slice(..).get_mapped_range().to_vec();
        readback.unmap();

        // Strip row padding: copy W*4 bytes from each padded row.
        let mut pixels: Vec<u8> = Vec::with_capacity((W * H * BYTES_PER_PIXEL) as usize);
        for row in 0..H as usize {
            let src = row * ROW_BYTES_PADDED as usize;
            pixels.extend_from_slice(&padded[src..src + ROW_BYTES_UNPADDED as usize]);
        }

        // Bgra8UnormSrgb bytes are [B, G, R, A]; PNG expects [R, G, B, A].
        // Swap in-place — no gamma conversion: sRGB values are written directly.
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        // Write PNG.
        image::save_buffer(
            out_path,
            &pixels,
            W,
            H,
            image::ColorType::Rgba8,
        )
        .unwrap_or_else(|e| panic!("failed to write {out_path}: {e}"));

        println!("wrote {out_path}  ({W}×{H})");
    }

    // --- grain-strength sweep: PaperLight at four grain multipliers ---
    crew_theme::set_theme(ThemeId::PaperLight);
    let panes = build_scene(cell_w, cell_h);
    cell_grid.set_scene(&device, &panes);
    cell_grid.prepare(&device, &queue, W, H);

    let bg = crew_theme::theme().page_bg;
    let bg_f32 = [
        bg.0 as f32 / 255.0,
        bg.1 as f32 / 255.0,
        bg.2 as f32 / 255.0,
        1.0_f32,
    ];

    const SCRATCHPAD: &str = "/private/tmp/claude-501/-Users-atyagi-code-crew/7f6ecec7-c641-4cc2-a923-a17dea6afba0/scratchpad";

    for grain_mul in [0.0_f32, 0.6, 1.0, 1.6] {
        let out_path = format!("{SCRATCHPAD}/grain-{grain_mul:.1}.png");

        paper_bg.update_uniform(&queue, bg_f32, W as f32, H as f32, 1.0, grain_mul);

        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("grain_sweep_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &tex_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg.0 as f64 / 255.0,
                            g: bg.1 as f64 / 255.0,
                            b: bg.2 as f64 / 255.0,
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
            paper_bg.draw(&mut pass);
            cell_grid.draw(&mut pass);
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
                    bytes_per_row: Some(ROW_BYTES_PADDED),
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
            .expect("poll failed");

        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("poll (map) failed");

        let padded = readback.slice(..).get_mapped_range().to_vec();
        readback.unmap();

        let mut pixels: Vec<u8> = Vec::with_capacity((W * H * BYTES_PER_PIXEL) as usize);
        for row in 0..H as usize {
            let src = row * ROW_BYTES_PADDED as usize;
            pixels.extend_from_slice(&padded[src..src + ROW_BYTES_UNPADDED as usize]);
        }

        // Bgra8UnormSrgb → RGBA8 swap
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        image::save_buffer(&out_path, &pixels, W, H, image::ColorType::Rgba8)
            .unwrap_or_else(|e| panic!("failed to write {out_path}: {e}"));

        println!("wrote {out_path}  (grain_mul={grain_mul:.1})");
    }

    // --- welcome screen: one full-canvas pane per theme ---
    for (theme_id, out_path) in [
        (
            ThemeId::PaperLight,
            format!("{SCRATCHPAD}/welcome-light.png"),
        ),
        (ThemeId::PaperDark, format!("{SCRATCHPAD}/welcome-dark.png")),
    ] {
        crew_theme::set_theme(theme_id);

        let cols = (W as f32 / cell_w).floor() as u16;
        let rows = (H as f32 / cell_h).floor() as u16;
        let panes = build_welcome_scene(cols, rows, 28, W as f32, H as f32);

        cell_grid.set_scene(&device, &panes);
        cell_grid.prepare(&device, &queue, W, H);

        let bg = crew_theme::theme().page_bg;
        let bg_f32 = [
            bg.0 as f32 / 255.0,
            bg.1 as f32 / 255.0,
            bg.2 as f32 / 255.0,
            1.0_f32,
        ];
        paper_bg.update_uniform(&queue, bg_f32, W as f32, H as f32, 1.0, 1.0);

        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("welcome_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &tex_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg.0 as f64 / 255.0,
                            g: bg.1 as f64 / 255.0,
                            b: bg.2 as f64 / 255.0,
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
            paper_bg.draw(&mut pass);
            cell_grid.draw(&mut pass);
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
                    bytes_per_row: Some(ROW_BYTES_PADDED),
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
            .expect("poll failed");
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("poll (map) failed");

        let padded = readback.slice(..).get_mapped_range().to_vec();
        readback.unmap();

        let mut pixels: Vec<u8> = Vec::with_capacity((W * H * BYTES_PER_PIXEL) as usize);
        for row in 0..H as usize {
            let src = row * ROW_BYTES_PADDED as usize;
            pixels.extend_from_slice(&padded[src..src + ROW_BYTES_UNPADDED as usize]);
        }
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }
        image::save_buffer(&out_path, &pixels, W, H, image::ColorType::Rgba8)
            .unwrap_or_else(|e| panic!("failed to write {out_path}: {e}"));
        println!("wrote {out_path}  ({W}×{H})");
    }
}

/// ASCII-art banner — Standard figlet style, all rows padded to equal width.
const WELCOME_BANNER: &[&str] = &[
    r"  ____   ____   _____  __        __",
    r" / ___| |  _ \ | ____| \ \      / /",
    r"| |     | |_) ||  _|    \ \ /\ / / ",
    r"| |___  |  _ < | |___    \ V  V /  ",
    r" \____| |_| \_||_____|    \_/\_/   ",
];
const WELCOME_BANNER_W: u16 = 35;
const WELCOME_BANNER_H: u16 = WELCOME_BANNER.len() as u16;
const WELCOME_TAGLINE: &str = "fast terminals. clean flow.";
const WELCOME_HINT: &str = "Cmd+T  new shell    ·    /  commands";
const WELCOME_PULSE: u64 = 56;
const WELCOME_SCENE_W: u16 = 26;
const WELCOME_SCENE_H: u16 = 3;
const WELCOME_BEAT: u64 = 8;

fn welcome_col_style(tick: u64, col: u16) -> ((u8, u8, u8), bool) {
    let phase = (tick / 2 + u64::from(col) * 3) % WELCOME_PULSE;
    let half = WELCOME_PULSE / 2;
    let dist = if phase < half {
        phase
    } else {
        WELCOME_PULSE - phase
    };
    let t = crew_theme::theme();
    let acc = t.accent_default;
    if dist == 0 {
        return (acc, true);
    }
    let frac = dist as f32 / half as f32;
    let lerp = |a: u8, b: u8| (a as f32 + frac * (b as f32 - a as f32)) as u8;
    let (ar, ag, ab) = acc;
    let (dr, dg, db) = t.text_muted;
    ((lerp(ar, dr), lerp(ag, dg), lerp(ab, db)), false)
}

/// Screenshot-harness copy of `crew-app::welcomeart::scene` (the "dev at the
/// terminal" worker). Pushes non-space glyphs; spaces let the page show through.
fn welcome_worker(
    cells: &mut Vec<CellView>,
    top: u16,
    left: u16,
    tick: u64,
    fg: (u8, u8, u8),
    accent: (u8, u8, u8),
    bg: (u8, u8, u8),
) {
    let f = (tick / WELCOME_BEAT) % 2;
    let hand = if f == 0 { '╯' } else { '╮' };
    let cur = if f == 0 { '▋' } else { ' ' };
    let bar = "─".repeat(14);
    let mut seg = |row: u16, x: u16, s: &str, color: (u8, u8, u8)| -> u16 {
        for (i, ch) in s.chars().enumerate() {
            if ch != ' ' {
                cells.push(CellView {
                    col: x + i as u16,
                    row,
                    c: ch,
                    fg: color,
                    bg,
                    bold: false,
                    italic: false,
                });
            }
        }
        x + s.chars().count() as u16
    };
    seg(top, left, &format!(" (•_•)    ┌{bar}┐"), fg);
    seg(top + 2, left, &format!(r" /    \   └{bar}┘"), fg);
    let y = top + 1;
    let mut x = left;
    x = seg(y, x, &format!("<)   ){hand}   │  "), fg);
    x = seg(y, x, "crew:~$", accent);
    x = seg(y, x, " ", fg);
    x = seg(y, x, &cur.to_string(), accent);
    seg(y, x, "   │", fg);
}

/// Build a welcome-screen scene: ASCII CREW banner centred on a `W×H` px canvas.
fn build_welcome_scene(cols: u16, rows: u16, tick: u64, pw: f32, ph: f32) -> Vec<PaneScene> {
    let t = crew_theme::theme();
    let bg = t.page_bg;
    let mut cells: Vec<CellView> = Vec::new();

    let use_banner = WELCOME_BANNER_W < cols && WELCOME_BANNER_H + 4 < rows;
    if use_banner {
        // Mirror crew-app::welcome: reserve room for the worker scene so the
        // whole stack stays centred. (crew-render can't depend on crew-app, so
        // the welcome art is duplicated here for the screenshot harness.)
        let scene_fits = WELCOME_SCENE_W < cols;
        let extra = if scene_fits { WELCOME_SCENE_H + 1 } else { 0 };
        let top = (rows.saturating_sub(WELCOME_BANNER_H + 4 + extra)) / 2;
        let left = (cols - WELCOME_BANNER_W) / 2;
        for (li, line) in WELCOME_BANNER.iter().enumerate() {
            let row = top + li as u16;
            if row >= rows {
                break;
            }
            for (ci, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let abs_col = left + ci as u16;
                if abs_col >= cols {
                    break;
                }
                let (fg, bold) = welcome_col_style(tick, ci as u16);
                cells.push(CellView {
                    col: abs_col,
                    row,
                    c: ch,
                    fg,
                    bg,
                    bold,
                    italic: false,
                });
            }
        }
        let tl_row = top + WELCOME_BANNER_H + 1;
        let tl_w = WELCOME_TAGLINE.chars().count() as u16;
        if tl_row < rows && tl_w < cols {
            let tl_start = (cols - tl_w) / 2;
            for (i, ch) in WELCOME_TAGLINE.chars().enumerate() {
                cells.push(CellView {
                    col: tl_start + i as u16,
                    row: tl_row,
                    c: ch,
                    fg: t.hint_fg,
                    bg,
                    bold: false,
                    italic: false,
                });
            }
        }
        let hint_row = tl_row + 1;
        let hint_w = WELCOME_HINT.chars().count() as u16;
        if hint_row < rows && hint_w < cols {
            let hs = (cols - hint_w) / 2;
            for (i, ch) in WELCOME_HINT.chars().enumerate() {
                cells.push(CellView {
                    col: hs + i as u16,
                    row: hint_row,
                    c: ch,
                    fg: t.hint_fg,
                    bg,
                    bold: false,
                    italic: false,
                });
            }
        }
        let sc_top = hint_row + 2;
        if scene_fits && sc_top + WELCOME_SCENE_H < rows {
            let sc_left = (cols - WELCOME_SCENE_W) / 2;
            welcome_worker(
                &mut cells,
                sc_top,
                sc_left,
                tick,
                t.text_muted,
                t.accent_default,
                bg,
            );
        }
    }
    // Version stamp bottom-right.
    let ver = "v0.5.11";
    let vw = ver.chars().count() as u16;
    if vw + 1 < cols {
        let vstart = cols - vw - 1;
        for (i, ch) in ver.chars().enumerate() {
            cells.push(CellView {
                col: vstart + i as u16,
                row: rows - 1,
                c: ch,
                fg: t.dim,
                bg,
                bold: false,
                italic: false,
            });
        }
    }
    vec![PaneScene {
        cells,
        x: 0.0,
        y: 0.0,
        w: pw,
        h: ph,
        focused: false,
        bordered: false,
        overlay: false,
    }]
}

/// Build a representative crew frame scene.
///
/// Layout (all px, 1200×800 canvas):
///   sidebar   x=8   y=8  w=200 h=590  unfocused  bordered
///   main term x=220 y=8  w=720 h=590  FOCUSED    bordered
///   aux term  x=952 y=8  w=240 h=590  unfocused  bordered
///   input bar x=8   y=612 w=1184 h=72  unfocused  bordered
fn build_scene(cell_w: f32, cell_h: f32) -> Vec<PaneScene> {
    let mut panes = Vec::new();

    // --- sidebar ---
    let sb_x = 8.0_f32;
    let sb_y = 8.0_f32;
    let sb_w = 200.0_f32;
    let sb_h = 590.0_f32;
    panes.push(PaneScene {
        cells: sidebar_cells(cell_w, cell_h, sb_w, sb_h),
        x: sb_x,
        y: sb_y,
        w: sb_w,
        h: sb_h,
        focused: false,
        bordered: true,
        overlay: false,
    });

    // --- main terminal (focused) ---
    let mt_x = 220.0_f32;
    let mt_y = 8.0_f32;
    let mt_w = 720.0_f32;
    let mt_h = 590.0_f32;
    panes.push(PaneScene {
        cells: terminal_cells_main(cell_w, cell_h, mt_w, mt_h),
        x: mt_x,
        y: mt_y,
        w: mt_w,
        h: mt_h,
        focused: true,
        bordered: true,
        overlay: false,
    });

    // --- auxiliary terminal (unfocused) ---
    let at_x = 952.0_f32;
    let at_y = 8.0_f32;
    let at_w = 240.0_f32;
    let at_h = 590.0_f32;
    panes.push(PaneScene {
        cells: terminal_cells_aux(cell_w, cell_h, at_w, at_h),
        x: at_x,
        y: at_y,
        w: at_w,
        h: at_h,
        focused: false,
        bordered: true,
        overlay: false,
    });

    // --- input bar ---
    panes.push(PaneScene {
        cells: input_bar_cells(cell_w, cell_h, 1184.0, 72.0),
        x: 8.0,
        y: 612.0,
        w: 1184.0,
        h: 72.0,
        focused: false,
        bordered: true,
        overlay: false,
    });

    panes
}

/// Place a string of cells starting at (col, row) with a given fg colour.
fn place_str(
    cells: &mut Vec<CellView>,
    row: u16,
    start_col: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) {
    for (i, c) in s.chars().enumerate() {
        cells.push(CellView {
            col: start_col + i as u16,
            row,
            c,
            fg,
            bg: crew_theme::theme().page_bg,
            bold,
            italic: false,
        });
    }
}

// --- sidebar: label/value stat rows ---
fn sidebar_cells(_cw: f32, _ch: f32, _w: f32, _h: f32) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut cells = Vec::new();
    let label_fg = t.text_muted;
    let value_fg = t.ink;

    let rows: &[(&str, &str)] = &[
        ("CPU   ", "12%"),
        ("MEM   ", "1.4 G"),
        ("LOAD  ", "0.82"),
        ("UPTIME", "3h 14m"),
        ("BRANCH", "feat/themes"),
        ("DIRTY ", "3 files"),
        ("AHEAD ", "+2"),
    ];

    for (i, (label, value)) in rows.iter().enumerate() {
        let row = (i * 2 + 1) as u16;
        place_str(&mut cells, row, 1, label, label_fg, false);
        place_str(&mut cells, row, 8, value, value_fg, true);
    }
    cells
}

// --- main terminal: simulated git status output ---
fn terminal_cells_main(_cw: f32, _ch: f32, _w: f32, _h: f32) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut cells = Vec::new();
    let ink = t.term_fg;
    let green = t.ansi[2];
    let red = t.ansi[1];
    let yellow = t.ansi[3];
    let dim = t.text_muted;

    // Row 0: prompt
    place_str(&mut cells, 0, 0, "crew@mbp", green, true);
    place_str(&mut cells, 0, 8, ":", ink, false);
    place_str(&mut cells, 0, 9, "~/code/crew", yellow, true);
    place_str(&mut cells, 0, 20, " $ ", ink, false);
    place_str(&mut cells, 0, 23, "git status", ink, true);

    // Row 1: blank
    // Row 2: branch
    place_str(&mut cells, 2, 0, "On branch ", ink, false);
    place_str(&mut cells, 2, 10, "feat/crew-paper-themes", green, true);

    // Row 3
    place_str(
        &mut cells,
        3,
        0,
        "Your branch is up to date with",
        ink,
        false,
    );
    place_str(&mut cells, 3, 31, " 'origin/main'", dim, false);

    // Row 5
    place_str(
        &mut cells,
        5,
        0,
        "Changes not staged for commit:",
        yellow,
        true,
    );

    // Rows 6-9: modified files
    let modified = [
        "crates/crew-render/src/cellgrid.rs",
        "crates/crew-render/src/renderer.rs",
        "crates/crew-render/src/textprep.rs",
        "crates/crew-render/src/lib.rs",
    ];
    for (i, path) in modified.iter().enumerate() {
        let row = (6 + i) as u16;
        place_str(&mut cells, row, 2, "modified:   ", red, false);
        place_str(&mut cells, row, 14, path, red, false);
    }

    // Row 11
    place_str(&mut cells, 11, 0, "Untracked files:", yellow, true);

    // Row 12-13: untracked
    let untracked = [
        "crates/crew-render/examples/screenshot.rs",
        ".superpowers/sdd/task-15-report.md",
    ];
    for (i, path) in untracked.iter().enumerate() {
        let row = (12 + i) as u16;
        place_str(&mut cells, row, 2, path, red, false);
    }

    // Row 15: summary
    place_str(&mut cells, 15, 0, "no changes added to commit", ink, false);
    place_str(&mut cells, 15, 27, " (use \"git add\" ...)", dim, true);

    // Row 17: next prompt
    place_str(&mut cells, 17, 0, "crew@mbp", green, true);
    place_str(&mut cells, 17, 8, ":", ink, false);
    place_str(&mut cells, 17, 9, "~/code/crew", yellow, true);
    place_str(&mut cells, 17, 20, " $ ", ink, false);

    cells
}

// --- aux terminal: simulated ls --color output ---
fn terminal_cells_aux(_cw: f32, _ch: f32, _w: f32, _h: f32) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut cells = Vec::new();
    let ink = t.term_fg;
    let green = t.ansi[2];
    let yellow = t.ansi[3];
    let blue = t.ansi[4];
    let dim = t.text_muted;

    // prompt
    place_str(&mut cells, 0, 0, "$ ", ink, false);
    place_str(&mut cells, 0, 2, "ls --color", ink, true);

    // directory listing
    let entries: &[(&str, (u8, u8, u8))] = &[
        ("Cargo.lock", ink),
        ("Cargo.toml", ink),
        ("LICENSE", yellow),
        ("README.md", ink),
        ("crates/", blue),
        ("target/", dim),
        (".claude/", dim),
        (".git/", dim),
        (".gitignore", ink),
        (".superpowers/", blue),
    ];
    for (i, (name, fg)) in entries.iter().enumerate() {
        let row = (2 + i) as u16;
        place_str(&mut cells, row, 0, name, *fg, false);
    }

    // second prompt
    place_str(&mut cells, 13, 0, "$ ", ink, false);
    place_str(&mut cells, 13, 2, "cargo build", green, true);
    place_str(&mut cells, 14, 0, "   Compiling crew-render", ink, false);
    place_str(&mut cells, 15, 0, "   Compiling crew-app", ink, false);
    place_str(&mut cells, 16, 0, "    Finished", green, true);
    place_str(&mut cells, 16, 12, " dev [unoptimized]", dim, false);

    cells
}

// --- input bar ---
fn input_bar_cells(_cw: f32, _ch: f32, _w: f32, _h: f32) -> Vec<CellView> {
    let placeholder = crew_theme::theme().placeholder;
    let mut cells = Vec::new();
    place_str(
        &mut cells,
        1,
        2,
        "Type a command or message\u{2026}",
        placeholder,
        false,
    );
    cells
}
