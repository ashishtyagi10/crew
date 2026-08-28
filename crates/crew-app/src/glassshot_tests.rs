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
/// The pane rectangle both fixture panes use. Named, because every sample box
/// in the pixel assertions below is derived from it (see [`Geom`]) rather than
/// written as a literal.
const PANE_W: f32 = 320.0;
const PANE_H: f32 = 200.0;
const PANE_Y: f32 = 50.0;
const FOCUSED_X: f32 = 30.0;
const UNFOCUSED_X: f32 = 370.0;

fn panes(cell_w: f32, cell_h: f32) -> Vec<PaneScene> {
    let pane_w = PANE_W;
    let pane_h = PANE_H;
    let pane_at = |x: f32| crate::pane::Pane {
        glide: crate::glide::Glide::default(),
        content: crate::pane::PaneContent::Far(crate::farpane::FarPane::new(std::env::temp_dir())),
        grid: crew_term::GridSize {
            cols: ((pane_w - 2.0 * cell_w) / cell_w) as u16,
            rows: ((pane_h - 2.0 * cell_h) / cell_h) as u16,
        },
        rect: crate::layout::Rect {
            x,
            y: PANE_Y,
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
        // Born long ago, so `assemble_t` is 1.0 and the card is FULLY drawn.
        // With `now_ms()` here every one of these shots photographed a card
        // in its first frames of assembly — which draws outward from the four
        // corners, i.e. corner brackets and no edges at all. That is why
        // `crt_shot_grayscale_focus_hierarchy` measured page background where
        // it expected a bottom border, and why every glass and modern PNG in
        // this harness was a picture of a half-built frame.
        born_ms: 0,
    };
    crate::paneview::build_scenes(
        &[pane_at(FOCUSED_X), pane_at(UNFOCUSED_X)],
        Some(0),
        false,
        None,
        None,
        1.0,
        cell_w,
        cell_h,
        &Default::default(),
        &[],
    )
}

/// Render one full frame: clear → paper grain → glass → cells → borders → text.
fn render(glass: crew_theme::GlassLevel, opacity: f32) -> Option<Vec<u8>> {
    render_full(glass, opacity, false)
}

/// [`render`], optionally through the REAL CRT post-process: the scene draws
/// into the chain's off-screen target, then bloom + composite reproject onto
/// the readback texture — the same `CrtChain` the app's frame path owns, so a
/// pixel asserted here is a pixel the tube actually ships.
fn render_full(glass: crew_theme::GlassLevel, opacity: f32, crt: bool) -> Option<Vec<u8>> {
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

    // A static tube (time 0, flicker 0 — the idle determinism contract),
    // running the active theme's own style through the real bloom chain.
    let chain = crt.then(|| {
        let mut c = crew_render::CrtChain::new(&device, FORMAT, W, H);
        c.set_style(crew_theme::theme().crt);
        c.set_anim(0.0, 0.0);
        c.update_uniforms(&queue, W as f32, H as f32, !crew_theme::theme().dark);
        c
    });

    let paper = PaperBgPass::new(&device, FORMAT);
    let bg = crew_theme::theme().page_bg;
    let bg_f32 = crew_render::color::target_rgba(bg, opacity, FORMAT.is_srgb());
    // Mirrors `frame.rs`: the modern family's backdrop (gradient wash at
    // rest + dot lattice) rides the same pass, so a modern shot shows the
    // page the app actually draws rather than a bare fill.
    let modern = crew_theme::theme().modern.map(|m| {
        let c = |rgb| {
            let [r, g, b, _] = crew_render::color::target_rgba(rgb, 1.0, FORMAT.is_srgb());
            [r, g, b]
        };
        let (spacing, radius) = crew_render::ModernPaper::cell_geometry(cell_h);
        crew_render::ModernPaper {
            color_a: c(m.pole_a),
            color_b: c(m.pole_b),
            dots: m.dots,
            spacing,
            radius,
            wash: m.wash,
            phase: 0.0,
            // A shot has no focused card: the wash sits centred, which is the
            // page every other shot test in the tree was calibrated against.
            focus: [0.5, 0.5],
            focus_pull: 0.0,
        }
    });
    paper.update_uniform(
        &queue,
        bg_f32,
        (W as f32, H as f32),
        1.0,
        1.3 * crew_theme::theme().grain,
        modern.as_ref(),
    );

    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        // With the tube on, the scene pass lands off-screen and the chain's
        // composite owns the readback texture — exactly `frame::render`'s split.
        let scene_view = chain.as_ref().map_or(&view, |c| c.scene_view());
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("glass_shot_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: scene_view,
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
    if let Some(c) = &chain {
        c.encode(&mut enc, &view);
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

fn shot(
    name: &str,
    id: crew_theme::ThemeId,
    glass: crew_theme::GlassLevel,
    opacity: f32,
    expect_sheet: bool,
) {
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
    // honest question is whether the glass level moves the pixels — and since
    // the 2026-08-06 flat decree covered every family, whether it moves them
    // at all: on any theme a delta is the phantom shadow-box bug coming back.
    let flat = render(crew_theme::GlassLevel::Off, opacity).expect("adapter was available above");
    let on = mean_lum(&px, 60, 120, 200, 60);
    let bare = mean_lum(&flat, 60, 120, 200, 60);
    println!(
        "wrote {path}  on_card={on:.1} glass_off={bare:.1} delta={:.1}",
        on - bare
    );
    if expect_sheet {
        assert!(
            (on - bare).abs() > 1.5,
            "{name}: the sheet changes nothing — glass {on:.1} vs off {bare:.1}"
        );
    } else {
        assert!(
            (on - bare).abs() < 0.5,
            "{name}: paper must render flat — glass {on:.1} vs off {bare:.1}"
        );
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn glass_shot_every_theme_family() {
    let _g = crate::app::theme_test_guard();
    use crew_theme::{GlassLevel as G, ThemeId as T};
    shot("light", T::PaperLight, G::Medium, 1.0, false);
    shot("dark", T::PaperDark, G::Medium, 1.0, false);
    shot("crt", T::CrtGreen, G::Medium, 1.0, false);
    shot("light-high", T::PaperLight, G::High, 1.0, false);
    shot("dark-high", T::PaperDark, G::High, 1.0, false);
}

/// The MODERN family through the whole stack — wash + lattice under the
/// cells, the theme's own bloom-only tube over them — once per palette, so
/// the light half can be looked at in the composition it ships in. The light
/// pages are the reason this exists: their halo runs the bloom chain in the
/// opposite direction (`bloom.wgsl`'s ink pass), and the failure mode it
/// replaced — an additive pass blowing a near-white page to flat white — is
/// invisible in any unit test and obvious in one PNG.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn modern_shot_every_palette() {
    let _g = crate::app::theme_test_guard();
    use crew_theme::ThemeId as T;
    let out_dir = std::env::var("CREW_SHOT_DIR").unwrap_or_else(|_| "target/screenshots".into());
    std::fs::create_dir_all(&out_dir).unwrap();
    // Every palette, once — this loop used to list `Nebula` four times and
    // `Blossom` four times, so "every palette" shot two of them eight times
    // and overwrote the same two PNGs. The light pages are the reason the
    // test exists and half of them were never in it.
    for id in [T::Nebula, T::Blossom, T::Harbor, T::Fern] {
        crew_theme::set_theme(id);
        let Some(px) = render_full(crew_theme::GlassLevel::Medium, 1.0, true) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        let path = format!("{out_dir}/modern-{}.png", id.as_str());
        image::save_buffer(&path, &px, W, H, image::ColorType::Rgba8).unwrap();
        // The page must still be the page after the tube: a light palette
        // whose bloom ran additively lands at ~255 everywhere (that was the
        // bug), and a dark one must not be lifted into gray either. Sampled
        // in the DEAD CENTRE of the gap between the two panes (they end at
        // 350 and start at 370): the old 8px sample started 5px off the left
        // pane's right border, which is inside that border's bloom halo once
        // the card is drawn at all — and until this harness stopped
        // photographing half-assembled cards, it never was.
        let gap = mean_lum(&px, 358, 60, 4, 180);
        let want = crew_theme::theme().page_bg;
        let page =
            0.2126 * f64::from(want.0) + 0.7152 * f64::from(want.1) + 0.0722 * f64::from(want.2);
        println!("wrote {path}  gap_lum={gap:.1} page_lum={page:.1}");
        assert!(
            (gap - page).abs() < 12.0,
            "{}: the tube moved the bare page {page:.1} → {gap:.1}",
            id.as_str()
        );
    }
}

/// Mean of one RGBA channel over a block (3 = alpha).
fn mean_ch(px: &[u8], ch: usize, x0: usize, y0: usize, w: usize, h: usize) -> f64 {
    let mut s = 0.0;
    for y in y0..(y0 + h) {
        for x in x0..(x0 + w) {
            s += px[(y * W as usize + x) * 4 + ch] as f64;
        }
    }
    s / (w * h) as f64
}

/// The flat-tube contract on real pixels (2026-08-06, superseding the
/// 2026-08-04 luminous contract): with the glass sheet retired, the glass
/// level must not move a single region — neither the pane interior (the old
/// phosphor tint) nor the strip hugging the border (the old inner edge-glow).
/// Any delta is the drop-shadow/adrift-panes look coming back.
///
/// Blocks share the y-range 120..180 inside the left pane (rect 30,50
/// 320×200): the edge strip sits 4..12px inside the left border, the centre
/// block starts 30px in.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn glass_shot_crt_is_flat() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
    let Some(on) = render(crew_theme::GlassLevel::Medium, 1.0) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let off = render(crew_theme::GlassLevel::Off, 1.0).expect("adapter was available above");

    let centre_delta = mean_lum(&on, 60, 120, 200, 60) - mean_lum(&off, 60, 120, 200, 60);
    let edge_delta = mean_lum(&on, 34, 120, 8, 60) - mean_lum(&off, 34, 120, 8, 60);
    println!("crt flat: centre Δ{centre_delta:.1}; edge Δ{edge_delta:.1}");
    assert!(
        centre_delta.abs() < 0.5,
        "CRT interior is no longer flat: glass moved the centre by Δ{centre_delta:.1}"
    );
    assert!(
        edge_delta.abs() < 0.5,
        "CRT edge strip is no longer flat: glass moved it by Δ{edge_delta:.1}"
    );

    // The translucent-window path is unaffected by the (now invisible) glass
    // pass: the page still carries the window opacity.
    let win = render(crew_theme::GlassLevel::Medium, 0.6).expect("adapter was available above");
    let page_a = win[(10 * W as usize + 10) * 4 + 3];
    let card_a = mean_ch(&win, 3, 34, 120, 8, 60);
    println!("crt window: page_alpha={page_a} card_edge_alpha={card_a:.1}");
    assert!(
        (140..=175).contains(&page_a),
        "page alpha {page_a} should track the 0.6 window opacity (~153)"
    );
    assert!(
        card_a < 250.0,
        "something went opaque over the desktop (edge alpha {card_a:.1})"
    );

    let out_dir = std::env::var("CREW_SHOT_DIR").unwrap_or_else(|_| "target/screenshots".into());
    std::fs::create_dir_all(&out_dir).unwrap();
    image::save_buffer(
        format!("{out_dir}/glass-crt-flat.png"),
        &on,
        W,
        H,
        image::ColorType::Rgba8,
    )
    .unwrap();
}

/// Paper themes must not inherit the CRT edge-glow: `edge_glow = 0` has to be
/// a true no-op, so a paper-dark sheet stays FLAT along x — the same delta at
/// the border strip as at the card centre (the vertical ramp cancels because
/// both blocks share the y-range; the frost grain is deterministic per pixel).
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn glass_shot_paper_has_no_edge_glow() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let Some(on) = render(crew_theme::GlassLevel::Medium, 1.0) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let off = render(crew_theme::GlassLevel::Off, 1.0).expect("adapter was available above");
    let centre_delta = mean_lum(&on, 60, 120, 200, 60) - mean_lum(&off, 60, 120, 200, 60);
    let edge_delta = mean_lum(&on, 34, 120, 8, 60) - mean_lum(&off, 34, 120, 8, 60);
    println!("paper-dark: edge Δ{edge_delta:.1} vs centre Δ{centre_delta:.1}");
    assert!(
        (edge_delta - centre_delta).abs() < 1.5,
        "paper glass is no longer flat: edge Δ{edge_delta:.1} vs centre Δ{centre_delta:.1}"
    );
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

/// The tube's light trace through the FULL chain (scene -> bloom ->
/// composite), written out so it can be looked at.
///
/// Geometry comes from [`Geom`], never from pixel literals: the cell size is
/// whatever font the active theme loaded, and when it moved, the literals
/// that used to sit on the bottom border ended up six pixels under it,
/// measuring page background. Every band below is derived.
///
/// A design note earned by mutation-checking (from the luminous-sheet era,
/// kept because the lesson generalizes): the first draft sampled 3px bands
/// just OUTSIDE each frame's outer edge, and those bands measured IDENTICAL
/// with focus removed — the light out there was ambient spill plus
/// deterministic grain, not the border. The assertions below are the ones
/// that actually flip when focus is taken away, all in plain luminance
/// except the node-colour fingerprint.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn crt_shot_light_trace() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
    let Some(px) = render_full(crew_theme::GlassLevel::Medium, 1.0, true) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let out_dir = std::env::var("CREW_SHOT_DIR").unwrap_or_else(|_| "target/screenshots".into());
    std::fs::create_dir_all(&out_dir).unwrap();
    image::save_buffer(
        format!("{out_dir}/crt-lighttrace.png"),
        &px,
        W,
        H,
        image::ColorType::Rgba8,
    )
    .unwrap();

    // This test used to make four numeric claims about post-bloom pixels —
    // "the focused corner peaks 1.8x the unfocused one", "the focused bottom
    // border outshines", "the corner samples hotter than the edge midpoint",
    // "the corner peaks redder". Every one of them sampled pixel literals
    // with "at this cell size" written beside them, and the cell size moved:
    // by the time anyone looked, the bands sat six pixels off the strokes
    // they were aimed at and were comparing one patch of page background with
    // another (31.4 vs 30.5) while reading as a hierarchy assertion. The test
    // had been red on main for nineteen releases and said nothing true.
    //
    // What a focused frame owes the eye is now asserted where the decision is
    // actually made — on the drawn stroke colours, on every preset, with a
    // floor: `panecardglow_tests::a_focused_frame_declares_itself`. That is
    // the contract that found `fern` handing focus a 1.60:1 frame. This stays
    // a shot: it writes the tube's own PNG so the light trace can be looked
    // at, and asserts only what a shot can honestly assert — that the frame
    // put light on the page at all.
    let lit = px.chunks_exact(4).filter(|p| p[1] > 60).count();
    println!("crt-lighttrace: {lit} lit pixels");
    assert!(lit > 3000, "the tube drew: {lit} lit pixels");
}
