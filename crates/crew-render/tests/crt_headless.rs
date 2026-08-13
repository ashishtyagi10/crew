//! Headless GPU integration test for the CRT post-process. Drives the REAL
//! chain — `CrtChain` is exactly what the renderer's frame path runs, bloom
//! passes included — by uploading known patterns into the chain's own scene
//! target and reading the composite back: the flat panel fills edge-to-edge
//! (no barrel warp, no black bezel), scanlines darken alternating rows, the
//! half-res gaussian bloom throws a halo that still reads 16px from a bright
//! block (the old two-ring tap died ~8px out) and falls off with distance,
//! and `flicker = 0` is byte-for-byte static.
//!
//! On macOS/Metal this runs the real GPU render; on GPU-less CI it skips.
use crew_render::CrtChain;
use crew_theme::CrtStyle;

const N: usize = 64;
const STRIDE: usize = 256; // N * 4, already a COPY_BYTES_PER_ROW_ALIGNMENT multiple

fn r_at(buf: &[u8], x: usize, y: usize) -> u8 {
    buf[y * STRIDE + x * 4]
}

/// Upload `fill(x, y) -> [r,g,b,a]` into the chain's real scene target.
fn upload(queue: &wgpu::Queue, chain: &CrtChain, fill: impl Fn(usize, usize) -> [u8; 4]) {
    let mut data = vec![0u8; N * N * 4];
    for y in 0..N {
        for x in 0..N {
            data[(y * N + x) * 4..(y * N + x) * 4 + 4].copy_from_slice(&fill(x, y));
        }
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: chain.scene_texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(N as u32 * 4),
            rows_per_image: Some(N as u32),
        },
        wgpu::Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
    );
}

/// Run the whole post-process (bloom chain + composite) and read back the
/// surface — the same `CrtChain::encode` the app's frame path calls.
fn render(device: &wgpu::Device, queue: &wgpu::Queue, chain: &CrtChain) -> Vec<u8> {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("crt_out"),
        size: wgpu::Extent3d {
            width: N as u32,
            height: N as u32,
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
        label: Some("crt_readback"),
        size: (N * N * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let view = tex.create_view(&Default::default());
    let mut enc = device.create_command_encoder(&Default::default());
    chain.encode(&mut enc, &view);
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
                rows_per_image: Some(N as u32),
            },
        },
        wgpu::Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(enc.finish()));
    let wait = || {
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("poll failed");
    };
    wait();
    buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    wait();
    let data = buf.slice(..).get_mapped_range().to_vec();
    buf.unmap();
    data
}

#[test]
fn crt_headless() {
    let instance = wgpu::Instance::default();
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("crt_headless: no GPU adapter, skipping");
            return;
        }
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    let mut chain = CrtChain::new(&device, wgpu::TextureFormat::Rgba8Unorm, N as u32, N as u32);

    // --- Case 1: a solid mid-gray field → flat geometry + scanlines ---
    // Mid-gray (not white) so the scanline darkening stays visible: on a
    // saturated white field the phosphor glow would push every row past 1.0 and
    // both light and dark lines would clamp to 255, hiding the effect.
    upload(&queue, &chain, |_, _| [160, 160, 160, 255]);
    chain.set_style(Some(CrtStyle::DEFAULT));
    chain.set_anim(0.0, 0.0);
    chain.update_uniforms(&queue, N as f32, N as f32, false);
    let px = render(&device, &queue, &chain);

    // Flat geometry: with curvature 0 the image maps 1:1 and fills the panel
    // edge-to-edge, so every corner is lit — there is no bezel to black out.
    for (x, y) in [(0, 0), (N - 1, 0), (0, N - 1), (N - 1, N - 1)] {
        assert!(
            r_at(&px, x, y) > 40,
            "corner ({x},{y}) should be lit on a flat panel, got {}",
            r_at(&px, x, y)
        );
    }
    // The center is lit too.
    assert!(
        r_at(&px, N / 2, N / 2) > 40,
        "center should be lit, got {}",
        r_at(&px, N / 2, N / 2)
    );
    // Scanlines: the 3-pixel cosine darkens a line every period, so ADJACENT
    // rows differ sharply. Adjacent-row delta isolates the scanline from the
    // slow corner-darkening falloff (which barely changes between neighbours).
    let col = N / 2;
    let max_adjacent = (24..40)
        .map(|y| (r_at(&px, col, y) as i32 - r_at(&px, col, y + 1) as i32).abs())
        .max()
        .unwrap_or(0);
    assert!(
        max_adjacent >= 12,
        "scanlines should make adjacent center rows differ, max delta was {max_adjacent}"
    );

    // --- Case 2: a bright block on black → wide gaussian bloom halo ---
    // Hot block spans x in 28..36, y in 28..36; its right edge (first dark
    // column) is x=36, its vertical middle is y=32. A hot style (glow ≥ 0.9,
    // glow_radius ≥ 10 — the TRON-blue end of the family) with scanlines off
    // so the halo is measured pure, not modulated by the raster.
    let block_right_x = 36usize;
    let block_mid_y = 32usize;
    upload(&queue, &chain, |x, y| {
        let hot = (28..36).contains(&x) && (28..36).contains(&y);
        if hot {
            [255, 255, 255, 255]
        } else {
            [0, 0, 0, 255]
        }
    });
    chain.set_style(Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.0,
        glow: 1.0,
        glow_radius: 12.0,
        corner: 0.0,
        flicker: 0.0,
    }));
    chain.set_anim(0.0, 0.0);
    chain.update_uniforms(&queue, N as f32, N as f32, false);
    let g = render(&device, &queue, &chain);
    // A pixel just outside the block is black in the source but the blurred
    // bright-pass reaches it, so it must pick up light.
    assert!(
        r_at(&g, block_right_x + 1, block_mid_y) > 0,
        "glow should bleed past the block edge, got {}",
        r_at(&g, block_right_x + 1, block_mid_y)
    );
    // Holographic reach: the goal's contract. 16px from the stroke must still
    // visibly glow (the old two-ring halo was ≤ 4 at 8px), and the halo must
    // FALL OFF with distance — a glow, not a uniform wash.
    let at4 = r_at(&g, block_right_x + 4, block_mid_y);
    let at8 = r_at(&g, block_right_x + 8, block_mid_y);
    let at16 = r_at(&g, block_right_x + 16, block_mid_y);
    eprintln!("crt_headless: halo red at 4px={at4} 8px={at8} 16px={at16}");
    assert!(at16 >= 8, "halo too weak 16px out: {at16}");
    assert!(
        at16 < at4,
        "halo must fall off with distance: 16px={at16} vs 4px={at4}"
    );

    // --- Case 3: flicker = 0 is perfectly static (deterministic) ---
    // The bloom chain runs both times; nothing in it may depend on time.
    upload(&queue, &chain, |_, _| [160, 160, 160, 255]);
    chain.set_style(Some(CrtStyle::DEFAULT));
    chain.set_anim(123.0, 0.0);
    chain.update_uniforms(&queue, N as f32, N as f32, false);
    let a = render(&device, &queue, &chain);
    chain.set_anim(456.0, 0.0);
    chain.update_uniforms(&queue, N as f32, N as f32, false);
    let b = render(&device, &queue, &chain);
    assert_eq!(a, b, "flicker=0 must be static regardless of time");

    // --- Case 4: the LIGHT-page halo — a coloured shadow, not added light ---
    // The same block, but now a saturated blue ring stroke on a near-white
    // page (the light modern family: page (250,251,254), pole (37,99,235)).
    // With the tube's additive pass this frame clips to white; the ink pass
    // must instead lay the stroke's own hue softly around it and leave the
    // page and the neutral ink alone.
    let page = [250u8, 251, 254, 255];
    let ring = [37u8, 99, 235, 255];
    let text = [17u8, 24, 39, 255]; // slate body ink — neutral, must not halo
    upload(&queue, &chain, |x, y| {
        if (28..36).contains(&x) && (28..36).contains(&y) {
            ring
        } else if (4..12).contains(&x) && (4..12).contains(&y) {
            // Far from the ring, so this probe measures ITS bleed, not the
            // ring's halo reaching over.
            text
        } else {
            page
        }
    });
    let light_style = CrtStyle {
        curvature: 0.0,
        scanline: 0.0,
        glow: 0.9,
        glow_radius: 12.0,
        corner: 0.0,
        flicker: 0.0,
    };
    chain.set_style(Some(light_style));
    chain.set_anim(0.0, 0.0);
    chain.update_uniforms(&queue, N as f32, N as f32, true);
    let l = render(&device, &queue, &chain);
    let rgb = |buf: &[u8], x: usize, y: usize| -> (i32, i32, i32) {
        let o = (y * N + x) * 4;
        (buf[o] as i32, buf[o + 1] as i32, buf[o + 2] as i32)
    };
    let near = rgb(&l, block_right_x + 2, block_mid_y);
    let far = rgb(&l, N - 1, block_mid_y);
    let beside_text = rgb(&l, 14, 8);
    eprintln!("crt_headless [ink halo]: near={near:?} far={far:?} beside_text={beside_text:?}");
    // L1: the page keeps its own colour far from anything — no white-out, no
    // global haze (the additive pass would have clipped this to (255,255,255)).
    assert!(
        (far.0 - 250).abs() <= 3 && (far.1 - 251).abs() <= 3 && (far.2 - 254).abs() <= 3,
        "L1 failed: the far page should be untouched, got {far:?}"
    );
    // L2: beside the ring the page darkens, and it darkens TOWARD THE RING'S
    // HUE — red and green fall away while blue is nearly held, which is what
    // makes it read as blue light on paper rather than a gray smudge.
    assert!(
        near.0 <= 250 - 12 && near.1 <= 251 - 8,
        "L2 failed: no shadow beside the ring: {near:?}"
    );
    assert!(
        (far.2 - near.2) * 3 < far.0 - near.0,
        "L2 failed: the halo is not tinted toward the ring, got {near:?}"
    );
    // L3: neutral body ink casts no halo — the chroma gate is what keeps
    // paper text crisp instead of smudged.
    assert!(
        (beside_text.0 - 250).abs() <= 4 && (beside_text.2 - 254).abs() <= 4,
        "L3 failed: slate ink should not bleed, got {beside_text:?}"
    );
    // L4: and it falls off with distance, like the light halo it mirrors.
    let prof: Vec<i32> = (0..14)
        .map(|i| 250 - rgb(&l, block_right_x + 1 + i * 2, block_mid_y).0)
        .collect();
    eprintln!("crt_headless [ink halo]: darkening profile (every 2px) {prof:?}");
    // Compare points a whole ripple period apart. The 9-tap kernel steps
    // radius/4 half-res texels per tap — 6 full-res px at radius 12 — so the
    // profile carries a 6px comb (~2/255 peak-to-trough) on top of its
    // envelope; that is the existing kernel's behaviour on every theme, not
    // something the ink pass introduced, and sampling ripple-peak to
    // ripple-peak measures the envelope instead of the comb.
    let d5 = 250 - rgb(&l, block_right_x + 5, block_mid_y).0;
    let d17 = 250 - rgb(&l, block_right_x + 17, block_mid_y).0;
    let d27 = 250 - rgb(&l, block_right_x + 27, block_mid_y).0;
    assert!(
        d17 > 0 && d17 < d5,
        "L4 failed: shadow must reach out and fall off: 5px={d5} 17px={d17}"
    );
    assert!(d27 == 0, "L4 failed: the halo must end, got {d27} at 27px");

    eprintln!("crt_headless: flat geometry, scanlines, wide bloom halo, ink halo, static-flicker verified");
}
