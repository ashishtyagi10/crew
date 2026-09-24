//! Headless GPU integration test for the frosted-glass card shader.
//!
//! Two things here cannot be checked by a unit test: that `glass.wgsl` actually
//! compiles (building the pipeline runs it through naga), and that the sheet it
//! draws has the shape the design calls for — brighter at the top than the
//! bottom, a specular hairline on the upper edge, and a shadow falling *below*
//! the card. All three are asserted against real rendered pixels.
//!
//! On macOS with Metal this finds an adapter and renders for real; in GPU-less
//! CI it skips rather than failing.
use crew_render::{GlassCard, GlassLayer};

const SIZE: u32 = 64;
/// Row stride of the readback buffer (COPY_BYTES_PER_ROW_ALIGNMENT).
const STRIDE: usize = 256;

/// The card under test: centred, leaving room for the shadow to fall outside.
const CARD_X: f32 = 16.0;
const CARD_Y: f32 = 16.0;
const CARD_W: f32 = 32.0;
const CARD_H: f32 = 32.0;

/// Mid grey, so a lightening fill and a darkening shadow are BOTH visible. On
/// black the shadow would be invisible and the test would silently prove
/// nothing about it.
const CLEAR: f64 = 128.0 / 255.0;

fn px(buf: &[u8], x: usize, y: usize) -> (u8, u8, u8, u8) {
    let o = y * STRIDE + x * 4;
    (buf[o], buf[o + 1], buf[o + 2], buf[o + 3])
}

/// Mean of the R channel over a small block, to average out the frost grain —
/// a single pixel could pass or fail on noise alone.
fn block_r(buf: &[u8], x: usize, y: usize, half: usize) -> f64 {
    let mut v = Vec::new();
    for yy in (y - half)..=(y + half) {
        for xx in (x - half)..=(x + half) {
            v.push(px(buf, xx, yy).0 as f64);
        }
    }
    v.iter().sum::<f64>() / v.len() as f64
}

fn card(alpha_top: f32, alpha_bottom: f32, highlight_alpha: f32, shadow_alpha: f32) -> GlassCard {
    GlassCard {
        x: CARD_X,
        y: CARD_Y,
        w: CARD_W,
        h: CARD_H,
        radius: 8.0,
        alpha_top,
        alpha_bottom,
        // No grain: this test is about the sheet's shape, and noise only
        // widens the error bars on every assertion below.
        noise: 0.0,
        tint: [1.0, 1.0, 1.0, 1.0],
        highlight: [1.0, 1.0, 1.0, 1.0],
        highlight_alpha,
        shadow_alpha,
        // No scan, no edge-glow: this test is about the resting paper sheet;
        // the glow gets its own test below.
        scan: -1.0,
        edge_glow: 0.0,
        lift: 0.0,
        glint: -1.0,
        notch: Default::default(),
    }
}

fn render(device: &wgpu::Device, queue: &wgpu::Queue, cards: &[GlassCard]) -> Vec<u8> {
    render_lit(device, queue, cards, (0.0, 0.0))
}

fn render_lit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cards: &[GlassCard],
    tilt: (f32, f32),
) -> Vec<u8> {
    let mut layer = GlassLayer::new(device, wgpu::TextureFormat::Rgba8Unorm);
    layer.set_cards(device, cards);
    layer.set_view(queue, SIZE as f32, SIZE as f32, tilt);

    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glass_test_tex"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("glass_readback"),
        size: (SIZE * SIZE * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let view = tex.create_view(&Default::default());
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("glass_test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: CLEAR,
                        g: CLEAR,
                        b: CLEAR,
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
        layer.draw(&mut rp);
    }
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(STRIDE as u32),
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
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
    buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("poll failed");
    let data = buf.slice(..).get_mapped_range().to_vec();
    buf.unmap();
    data
}

/// The scan sweep must actually reach the pixels — a shader uniform that is
/// plumbed but never read looks exactly like a working feature from Rust, which
/// is precisely how v0.7.0 shipped glass that drew nothing.
#[test]
fn glass_scan_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("glass_scan_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    // A flat sheet, so any difference is the scan and not the top-down ramp.
    let flat = |scan: f32| GlassCard {
        scan,
        ..card(0.30, 0.30, 0.0, 0.0)
    };
    let none = render(&device, &queue, &[flat(-1.0)]);
    // The diagonal sheen centred on the card's middle column a quarter of the
    // way down: there `(across + down) / 2` is 0.375, which the sheen's
    // off-card-to-off-card travel (-0.12 → 1.12) reaches at 0.4.
    let swept = render(&device, &queue, &[flat(0.4)]);

    let y_band = (CARD_Y + CARD_H * 0.25) as usize;
    let y_far = (CARD_Y + CARD_H * 0.85) as usize;
    let x = (CARD_X + CARD_W / 2.0) as usize;

    let (band_off, band_on) = (block_r(&none, x, y_band, 2), block_r(&swept, x, y_band, 2));
    let (far_off, far_on) = (block_r(&none, x, y_far, 2), block_r(&swept, x, y_far, 2));
    println!("band {band_off:.1} -> {band_on:.1};  far {far_off:.1} -> {far_on:.1}");

    assert!(
        band_on > band_off + 2.0,
        "the scan did not brighten the band it passes over ({band_off:.1} -> {band_on:.1})"
    );
    assert!(
        (far_on - far_off).abs() < 2.0,
        "the scan leaked across the whole card ({far_off:.1} -> {far_on:.1})"
    );
}

/// The inner edge-glow must reach the pixels too — same lesson as the scan:
/// a field packed but never read by the shader is invisible from Rust. A flat
/// sheet (no ramp, no highlight) isolates the glow as the only gradient.
#[test]
fn glass_edge_glow_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("glass_edge_glow_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    let flat = |glow: f32| GlassCard {
        edge_glow: glow,
        lift: 0.0,
        glint: -1.0,
        ..card(0.30, 0.30, 0.0, 0.0)
    };
    let none = render(&device, &queue, &[flat(0.0)]);
    let lit = render(&device, &queue, &[flat(0.5)]);

    // Centre row, 4px inside the left edge vs the card centre (16px in).
    let y = (CARD_Y + CARD_H / 2.0) as usize;
    let x_edge = (CARD_X + 4.0) as usize;
    let x_mid = (CARD_X + CARD_W / 2.0) as usize;

    let (edge_off, mid_off) = (block_r(&none, x_edge, y, 1), block_r(&none, x_mid, y, 1));
    let (edge_on, mid_on) = (block_r(&lit, x_edge, y, 1), block_r(&lit, x_mid, y, 1));
    println!("edge {edge_off:.1} -> {edge_on:.1};  mid {mid_off:.1} -> {mid_on:.1}");

    // Zero strength is a true no-op: the flat sheet stays flat.
    assert!(
        (edge_off - mid_off).abs() < 1.5,
        "glow=0 still shades the edge ({edge_off:.1} vs {mid_off:.1})"
    );
    // With glow on, the border-adjacent fill outshines the card centre.
    assert!(
        edge_on > mid_on + 10.0,
        "no inward gradient from the edge ({edge_on:.1} vs mid {mid_on:.1})"
    );
    // And the glow brightens — it never darkens or erases the base fill.
    assert!(
        mid_on >= mid_off - 1.0,
        "glow dimmed the card centre ({mid_off:.1} -> {mid_on:.1})"
    );
}

#[test]
fn glass_headless() {
    let instance = wgpu::Instance::default();
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("glass_headless: no GPU adapter, skipping");
            return;
        }
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    let base = CLEAR * 255.0;

    // --- the full sheet -----------------------------------------------------
    // Building the layer above already compiled glass.wgsl through naga; if the
    // shader were invalid we would never reach these assertions.
    let full = render(&device, &queue, &[card(0.40, 0.15, 0.60, 0.40)]);

    // A1: the fill tints the card's interior. A white sheet over grey lightens.
    let inside = block_r(&full, 32, 32, 2);
    eprintln!("glass_headless: base={base:.1} inside={inside:.1}");
    assert!(
        inside > base + 8.0,
        "A1: interior {inside:.1} not lightened over base {base:.1}"
    );

    // A2: the top-lit ramp — the sheet is brighter at its top than its bottom.
    // Sampled a few px inside each edge so the specular band and the AA edge
    // don't dominate either reading.
    let top = block_r(&full, 32, 22, 2);
    let bottom = block_r(&full, 32, 43, 2);
    eprintln!("glass_headless: top={top:.1} bottom={bottom:.1}");
    assert!(
        top > bottom + 4.0,
        "A2: gradient not top-lit (top {top:.1} vs bottom {bottom:.1})"
    );

    // A3: the specular hairline — the brightest point on the card is the strip
    // hugging its top edge, not its middle.
    let hairline = block_r(&full, 32, 17, 0);
    eprintln!("glass_headless: hairline={hairline:.1}");
    assert!(
        hairline > top,
        "A3: no specular edge (hairline {hairline:.1} vs top {top:.1})"
    );

    // A4: the shadow falls BELOW the card and darkens the page there. This is
    // the assertion that fails if the quad padding ever stops covering the
    // blur, or the shadow offset flips sign.
    let under = block_r(&full, 32, 53, 1);
    eprintln!("glass_headless: under={under:.1}");
    assert!(
        under < base - 4.0,
        "A4: no drop shadow below the card (under {under:.1} vs base {base:.1})"
    );

    // A5: the shadow FALLS OFF with distance rather than washing the frame.
    // Not "the far corner is untouched" — with a 14px blur under a 32px card,
    // nothing in a 64px frame is genuinely far, and a test that demanded an
    // untouched corner would only be measuring the frame size. The property
    // that actually matters is the decay: the darkening far from the card must
    // be a small fraction of the darkening directly beneath it.
    let far = px(&full, 1, 1).0 as f64;
    let near_drop = base - under;
    let far_drop = base - far;
    eprintln!("glass_headless: near_drop={near_drop:.1} far_drop={far_drop:.1}");
    assert!(
        far_drop < near_drop * 0.30,
        "A5: shadow barely decays (far {far_drop:.1} vs near {near_drop:.1})"
    );

    // --- nothing to draw ----------------------------------------------------
    // B1: no cards → the page is exactly the clear colour.
    let empty = render(&device, &queue, &[]);
    for (i, c) in empty.chunks(4).take((SIZE * SIZE) as usize).enumerate() {
        assert_eq!(
            c[0] as f64, base,
            "B1: pixel {i} drawn with no cards submitted"
        );
    }

    // B2: a fully transparent card must also leave the page untouched — this is
    // what Glass=off renders, and it must cost nothing visually.
    let clear_card = render(&device, &queue, &[card(0.0, 0.0, 0.0, 0.0)]);
    let mid = px(&clear_card, 32, 32).0 as f64;
    assert_eq!(mid, base, "B2: zero-alpha card still tinted the page");

    // --- shadow is separable ------------------------------------------------
    // C1: with the shadow off, the region below the card is clean page. Proves
    // A4's darkening came from the shadow term and not from the fill spilling
    // outside the rounded rect.
    let no_shadow = render(&device, &queue, &[card(0.40, 0.15, 0.60, 0.0)]);
    let under_ns = block_r(&no_shadow, 32, 53, 1);
    eprintln!("glass_headless: under_no_shadow={under_ns:.1}");
    assert!(
        (under_ns - base).abs() <= 2.0,
        "C1: fill leaked outside the card ({under_ns:.1} vs base {base:.1})"
    );

    // --- rounded corners ----------------------------------------------------
    // D1: the card is a ROUNDED rect — its sharp corner is outside the shape,
    // so the fill must not reach it. With radius 8 the pixel just inside the
    // bounding box corner sits beyond the arc.
    let corner = px(&no_shadow, 17, 17).0 as f64;
    let edge_mid = block_r(&no_shadow, 32, 20, 1);
    eprintln!("glass_headless: corner={corner:.1} edge_mid={edge_mid:.1}");
    assert!(
        corner < edge_mid,
        "D1: corners are not rounded (corner {corner:.1} vs edge {edge_mid:.1})"
    );
}

/// Lift must reach the pixels: a focused (lifted) card throws a deeper shadow
/// below itself than the same card at rest, and a brighter rim.
#[test]
fn glass_lift_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("glass_lift_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");
    let at = |lift: f32| GlassCard {
        lift,
        ..card(0.40, 0.15, 0.40, 0.30)
    };
    let rest = render(&device, &queue, &[at(0.0)]);
    let up = render(&device, &queue, &[at(1.0)]);
    let (under_rest, under_up) = (block_r(&rest, 32, 55, 1), block_r(&up, 32, 55, 1));
    let (rim_rest, rim_up) = (block_r(&rest, 32, 17, 0), block_r(&up, 32, 17, 0));
    eprintln!("glass_lift_headless: under {under_rest:.1} -> {under_up:.1}; rim {rim_rest:.1} -> {rim_up:.1}");
    assert!(
        under_up < under_rest - 2.0,
        "lift did not deepen the shadow ({under_rest:.1} -> {under_up:.1})"
    );
    assert!(
        rim_up > rim_rest + 1.0,
        "lift did not brighten the rim ({rim_rest:.1} -> {rim_up:.1})"
    );
}

/// Lift thickens the sheet: a white tint at a dark theme's faint alpha comes
/// out brighter mid-card when the card has lifted — the one depth cue a dark
/// page shows, since a shadow on near-black draws nothing.
#[test]
fn glass_lift_thickens_the_sheet_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("glass_lift_thickens_the_sheet_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");
    let at = |lift: f32| GlassCard {
        lift,
        ..card(0.10, 0.10, 0.0, 0.0)
    };
    let rest = block_r(&render(&device, &queue, &[at(0.0)]), 32, 36, 1);
    let up = block_r(&render(&device, &queue, &[at(1.5)]), 32, 36, 1);
    eprintln!("glass_lift_thickens_the_sheet_headless: mid {rest:.1} -> {up:.1}");
    assert!(
        up > rest + 5.0,
        "lift left the sheet as thin ({rest:.1} -> {up:.1})"
    );
}

/// A well (negative lift) casts nothing onto the page and shades its own top
/// lip instead: dark just inside the top edge, clean page below the card.
#[test]
fn glass_well_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("glass_well_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");
    let base = CLEAR * 255.0;
    let well = render(
        &device,
        &queue,
        &[GlassCard {
            lift: -1.0,
            ..card(0.0, 0.0, 0.0, 0.30)
        }],
    );
    let (lip, mid, under) = (
        block_r(&well, 32, 19, 1),
        block_r(&well, 32, 36, 1),
        block_r(&well, 32, 55, 1),
    );
    eprintln!("glass_well_headless: lip={lip:.1} mid={mid:.1} under={under:.1}");
    assert!(
        lip < mid - 4.0,
        "no inner shadow under the lip ({lip:.1} vs {mid:.1})"
    );
    assert!(
        (under - base).abs() <= 1.0,
        "a well cast a shadow outside ({under:.1})"
    );
}

/// The focus glint brightens the top rim where it has run to, and only there.
#[test]
fn glass_glint_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("glass_glint_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");
    let at = |glint: f32| GlassCard {
        glint,
        ..card(0.10, 0.10, 0.20, 0.0)
    };
    let rest = render(&device, &queue, &[at(-1.0)]);
    let mid = render(&device, &queue, &[at(0.5)]);
    let (top_rest, top_mid) = (block_r(&rest, 32, 17, 0), block_r(&mid, 32, 17, 0));
    let (low_rest, low_mid) = (block_r(&rest, 32, 40, 1), block_r(&mid, 32, 40, 1));
    eprintln!("glass_glint_headless: top {top_rest:.1} -> {top_mid:.1}; body {low_rest:.1} -> {low_mid:.1}");
    assert!(
        top_mid > top_rest + 10.0,
        "the glint did not light the top rim"
    );
    assert!(
        (low_mid - low_rest).abs() < 1.0,
        "the glint leaked into the body"
    );
}

/// The pointer's tilt leans the rim light: a pointer at the lower right
/// brightens the right edge's rim and dims the left's.
#[test]
fn glass_tilt_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("glass_tilt_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");
    let cards = [card(0.10, 0.10, 0.60, 0.0)];
    let rest = render_lit(&device, &queue, &cards, (0.0, 0.0));
    let lit = render_lit(&device, &queue, &cards, (1.0, 1.0));
    let (l0, l1) = (block_r(&rest, 17, 32, 0), block_r(&lit, 17, 32, 0));
    let (r0, r1) = (block_r(&rest, 46, 32, 0), block_r(&lit, 46, 32, 0));
    eprintln!("glass_tilt_headless: left {l0:.1} -> {l1:.1}; right {r0:.1} -> {r1:.1}");
    assert!(r1 > r0 + 5.0, "the tilt did not light the right rim");
    assert!(l1 < l0 - 5.0, "the tilt did not dim the left rim");
}

/// The bevel: with a shadow to scale it, the edge facing away from the light
/// (the bottom) is shaded darker than the same card casting none.
#[test]
fn glass_bevel_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("glass_bevel_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");
    let flat = render(&device, &queue, &[card(0.4, 0.4, 0.5, 0.0)]);
    let bevel = render(&device, &queue, &[card(0.4, 0.4, 0.5, 0.4)]);
    let (b0, b1) = (block_r(&flat, 40, 46, 0), block_r(&bevel, 40, 46, 0));
    let (m0, m1) = (block_r(&flat, 32, 32, 1), block_r(&bevel, 32, 32, 1));
    eprintln!("glass_bevel_headless: bottom lip {b0:.1} -> {b1:.1}; centre {m0:.1} -> {m1:.1}");
    assert!(b1 < b0 - 4.0, "no shade on the far lip");
    assert!((m1 - m0).abs() < 1.0, "the shade reached the centre");
}

/// A legend's notch cuts the rim where the words stand and nowhere else —
/// a fieldset's legend. The sheet stays whole under the words (clearing it
/// left every title in a dark hole) and stops at its own edge (a tab of
/// glass raised over them read as a folder tab stuck on the card).
#[test]
fn glass_notch_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("glass_notch_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");
    let base = CLEAR * 255.0;
    let mut notch = crew_render::notch::Notch {
        depth: 6.0,
        ..Default::default()
    };
    // Card-left px 10..20 is the legend: page x 26..36.
    notch.top[0] = [10.0, 20.0];
    let cut = render(
        &device,
        &queue,
        &[GlassCard {
            notch,
            ..card(0.30, 0.30, 0.60, 0.0)
        }],
    );
    let (gap, beside) = (block_r(&cut, 31, 17, 0), block_r(&cut, 42, 17, 0));
    let (over, off) = (block_r(&cut, 31, 13, 0), block_r(&cut, 42, 13, 0));
    eprintln!(
        "glass_notch_headless: gap {gap:.1} beside {beside:.1} over {over:.1} off {off:.1} page {base:.1}"
    );
    assert!(
        gap > base + 5.0,
        "the legend stands on bare page ({gap:.1})"
    );
    assert!(
        gap < beside - 10.0,
        "the rim still crosses the legend ({gap:.1} vs {beside:.1})"
    );
    // Above the edge, over the legend or beside it, is page: no tab.
    assert!(
        (over - off).abs() <= 2.0,
        "the sheet rose over the legend ({over:.1} vs {off:.1})"
    );
    // Below the legend's row the sheet is whole again.
    let under = block_r(&cut, 31, 28, 1);
    assert!(under > base + 10.0, "the notch ate the card ({under:.1})");
}
