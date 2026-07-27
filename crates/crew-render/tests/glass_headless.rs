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
    }
}

fn render(device: &wgpu::Device, queue: &wgpu::Queue, cards: &[GlassCard]) -> Vec<u8> {
    let mut layer = GlassLayer::new(device, wgpu::TextureFormat::Rgba8Unorm);
    layer.set_cards(device, cards);
    layer.set_viewport(queue, SIZE as f32, SIZE as f32);

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
    // what `/glass off` renders, and it must cost nothing visually.
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
