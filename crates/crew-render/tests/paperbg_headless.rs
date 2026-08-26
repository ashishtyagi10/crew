/// Headless GPU integration test for the paper-grain background shader.
///
/// On macOS with Metal this will find an adapter and run the real GPU render.
/// In GPU-less CI the test gracefully skips instead of failing.
use crew_render::{ModernPaper, PaperBgPass};

/// Read the R channel of pixel (x, y) from a tightly-packed 256-byte-stride RGBA buffer.
fn pixel_r(buf: &[u8], x: usize, y: usize) -> u8 {
    buf[y * 256 + x * 4]
}

/// Per-pixel R stddev over the flat 24..40 centre block (vignette-flat).
fn centre_stddev(buf: &[u8]) -> f64 {
    let mut vals = Vec::new();
    for y in 24..40 {
        for x in 24..40 {
            vals.push(pixel_r(buf, x, y) as f64);
        }
    }
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    (vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64).sqrt()
}

/// Stddev of the same block after 2x2 box-downsampling: white noise
/// collapses (~/2), coarse structure survives — the "fiber present" signal.
fn downsampled_stddev(buf: &[u8]) -> f64 {
    let mut vals = Vec::new();
    for y in (24..40).step_by(2) {
        for x in (24..40).step_by(2) {
            let s = pixel_r(buf, x, y) as f64
                + pixel_r(buf, x + 1, y) as f64
                + pixel_r(buf, x, y + 1) as f64
                + pixel_r(buf, x + 1, y + 1) as f64;
            vals.push(s / 4.0);
        }
    }
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    (vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64).sqrt()
}

fn render_64x64(device: &wgpu::Device, queue: &wgpu::Queue, pass: &PaperBgPass) -> Vec<u8> {
    render_offscreen(device, queue, pass, 64, 64)
}

/// Render into an off-screen `w`×`h` texture and return the readback rows.
/// `w` must keep the row stride (`w * 4`) a multiple of 256, wgpu's copy
/// alignment — 64 and 128 both do.
fn render_offscreen(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pass: &PaperBgPass,
    w: u32,
    h: u32,
) -> Vec<u8> {
    // Offscreen texture.
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test_tex"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    // Readback buffer: w×h×4 bytes, one row per `w * 4` (a multiple of
    // COPY_BYTES_PER_ROW_ALIGNMENT, so no padding).
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(w * h * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let view = tex.create_view(&Default::default());
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.draw(&mut rp);
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
                bytes_per_row: Some(w * 4),
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
fn paperbg_headless() {
    // --- adapter ---
    let instance = wgpu::Instance::default();
    let adapter_result =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            compatible_surface: None,
            force_fallback_adapter: false,
        }));
    let adapter = match adapter_result {
        Ok(a) => a,
        Err(_) => {
            eprintln!("paperbg_headless: no GPU adapter, skipping");
            return;
        }
    };

    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    // --- build pass (also validates the WGSL via naga) ---
    let paper_bg = PaperBgPass::new(&device, wgpu::TextureFormat::Rgba8Unorm);

    // Set PaperLight theme: page_bg = (244, 241, 234).
    crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
    let bg_f32 = [244.0_f32 / 255.0, 241.0_f32 / 255.0, 234.0_f32 / 255.0, 1.0];

    // -------------------------------------------------------
    // Case 1: grain+vignette enabled (intensity=1.0, grain_mul=1.0)
    // -------------------------------------------------------
    paper_bg.update_uniform(&queue, bg_f32, (64.0, 64.0), 1.0, 1.0, None);
    let pixels = render_64x64(&device, &queue, &paper_bg);

    // Helper to read pixel (x, y) → (R, G, B, A).
    let px = |x: usize, y: usize| -> (u8, u8, u8, u8) {
        let off = y * 256 + x * 4;
        (
            pixels[off],
            pixels[off + 1],
            pixels[off + 2],
            pixels[off + 3],
        )
    };

    let (centre_r, _, _, _) = px(32, 32);
    let (c0r, _, _, _) = px(0, 0);
    let (c1r, _, _, _) = px(63, 0);
    let (c2r, _, _, _) = px(0, 63);
    let (c3r, _, _, _) = px(63, 63);
    let avg_corner_r = (c0r as f32 + c1r as f32 + c2r as f32 + c3r as f32) / 4.0;

    // Variance: max R - min R across all pixels.
    let (max_r, min_r) = pixels
        .chunks(4)
        .map(|p| p[0])
        .fold((0u8, 255u8), |(mx, mn), r| (mx.max(r), mn.min(r)));

    eprintln!(
        "paperbg_headless [intensity=1 grain_mul=1]: centre R={} corner_avg_R={:.1} max_R={} min_R={}",
        centre_r, avg_corner_r, max_r, min_r
    );

    // A1: centre ≈ bg (244 ± 10).
    assert!(
        (centre_r as i32 - 244).abs() <= 10,
        "A1 failed: centre R={centre_r} expected within 10 of 244"
    );

    // A2: corner darker than centre. The +5 margin is stricter than a bare
    // "<" so grain noise can't make a corner spuriously pass; observed
    // darkening from the ~5% vignette is ~8 units, leaving headroom.
    assert!(
        (avg_corner_r as i32) + 5 < centre_r as i32,
        "A2 failed: avg_corner_r={avg_corner_r:.1} should be darker than centre_r={centre_r}"
    );

    // A3: some variance exists (grain produces non-uniform output).
    assert!(
        max_r > min_r,
        "A3 failed: max_R={max_r} min_R={min_r} — expected variance > 0"
    );

    // -------------------------------------------------------
    // Case 2: flat (intensity=0.0, grain_mul=1.0) — output = page_bg exactly
    // -------------------------------------------------------
    paper_bg.update_uniform(&queue, bg_f32, (64.0, 64.0), 0.0, 1.0, None);
    let flat_pixels = render_64x64(&device, &queue, &paper_bg);

    let (flat_r, flat_g, flat_b, _) = {
        let p = &flat_pixels;
        (p[0], p[1], p[2], p[3])
    };
    eprintln!("paperbg_headless [intensity=0]: first pixel R={flat_r} G={flat_g} B={flat_b}");

    for (i, chunk) in flat_pixels.chunks(4).enumerate() {
        let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
        assert!(
            (r as i32 - 244).abs() <= 1,
            "B1 failed pixel {i}: R={r} expected ~244"
        );
        assert!(
            (g as i32 - 241).abs() <= 1,
            "B1 failed pixel {i}: G={g} expected ~241"
        );
        assert!(
            (b as i32 - 234).abs() <= 1,
            "B1 failed pixel {i}: B={b} expected ~234"
        );
    }

    // -------------------------------------------------------
    // Case 3: grain_mul=0.0 (intensity=1.0) — no grain, only vignette
    // -------------------------------------------------------
    paper_bg.update_uniform(&queue, bg_f32, (64.0, 64.0), 1.0, 0.0, None);
    let nograin_pixels = render_64x64(&device, &queue, &paper_bg);

    let nograin_centre_r = pixel_r(&nograin_pixels, 32, 32);
    let nograin_c0r = pixel_r(&nograin_pixels, 0, 0);
    let nograin_c1r = pixel_r(&nograin_pixels, 63, 0);
    let nograin_c2r = pixel_r(&nograin_pixels, 0, 63);
    let nograin_c3r = pixel_r(&nograin_pixels, 63, 63);
    let nograin_avg_corner =
        (nograin_c0r as f32 + nograin_c1r as f32 + nograin_c2r as f32 + nograin_c3r as f32) / 4.0;

    eprintln!(
        "paperbg_headless [intensity=1 grain_mul=0]: centre R={nograin_centre_r} corner_avg_R={nograin_avg_corner:.1}"
    );

    // Centre should be close to bg (no grain, vignette at centre = 1.0).
    assert!(
        (nograin_centre_r as i32 - 244).abs() <= 2,
        "C1 failed: nograin centre R={nograin_centre_r} expected ~244"
    );
    // Corners still darker from vignette.
    assert!(
        (nograin_avg_corner as i32) + 5 < nograin_centre_r as i32,
        "C2 failed: nograin corner {nograin_avg_corner:.1} not darker than centre {nograin_centre_r}"
    );

    // -------------------------------------------------------
    // Case 4: grain_mul=2.4 (intensity=1.0) — the max reachable newsprint
    // value. Effective grain is `paper_grain * theme.grain`; `paper_grain`
    // (the user knob) tops out at 2.0 and light themes' `grain` field is 1.2
    // (vs 1.0 on dark themes, recalibrated for gamma-space blending — see
    // paperbg.wgsl), so 2.0 * 1.2 = 2.4 is the upper end reachable in the
    // app. This exercises that ceiling: variance should be strictly wider
    // than at grain_mul=1.0 on the same page colour (Case 1, `max_r`/`min_r`
    // above).
    // -------------------------------------------------------
    paper_bg.update_uniform(&queue, bg_f32, (64.0, 64.0), 1.0, 2.4, None);
    let strong_pixels = render_64x64(&device, &queue, &paper_bg);

    let (strong_max_r, strong_min_r) = strong_pixels
        .chunks(4)
        .map(|p| p[0])
        .fold((0u8, 255u8), |(mx, mn), r| (mx.max(r), mn.min(r)));
    let spread_1 = max_r as i32 - min_r as i32;
    let spread_24 = strong_max_r as i32 - strong_min_r as i32;

    eprintln!(
        "paperbg_headless [intensity=1 grain_mul=2.4]: max_R={strong_max_r} min_R={strong_min_r} spread={spread_24} (grain_mul=1 spread={spread_1})"
    );

    // D1: stronger grain multiplier widens the pixel spread.
    assert!(
        spread_24 > spread_1,
        "D1 failed: grain_mul=2.4 spread={spread_24} should exceed grain_mul=1 spread={spread_1}"
    );

    // -------------------------------------------------------
    // Case 5: dark page (8,8,8) at the app's default dark drive
    // (grain_mul = 1.56 = knob 1.3 × theme.grain 1.2): newsprint-band
    // per-pixel spread AND surviving coarse structure after 2x2 downsample.
    // -------------------------------------------------------
    let dark_bg = [8.0_f32 / 255.0, 8.0 / 255.0, 8.0 / 255.0, 1.0];
    paper_bg.update_uniform(&queue, dark_bg, (64.0, 64.0), 1.0, 1.56, None);
    let dark_pixels = render_64x64(&device, &queue, &paper_bg);
    let dark_std = centre_stddev(&dark_pixels);
    let dark_coarse = downsampled_stddev(&dark_pixels);
    eprintln!("paperbg_headless dark: std={dark_std:.2} coarse={dark_coarse:.2}");
    assert!(
        (4.0..=9.0).contains(&dark_std),
        "dark grain out of newsprint band: {dark_std:.2}"
    );
    assert!(
        dark_coarse >= dark_std * 0.55,
        "no fiber structure: coarse {dark_coarse:.2} vs fine {dark_std:.2} — \
         pure white noise would collapse to ~0.5x under 2x2 downsampling"
    );

    // Light page must stay fiber-free: downsampled spread collapses like
    // white noise (octave is dark_weight-gated).
    let light_std = centre_stddev(&pixels);
    let light_coarse = downsampled_stddev(&pixels);
    eprintln!("paperbg_headless light: std={light_std:.2} coarse={light_coarse:.2}");
    assert!(
        light_coarse <= light_std * 0.7,
        "light page grew coarse structure: {light_coarse:.2} vs {light_std:.2}"
    );

    // -------------------------------------------------------
    // Case 6: the modern dot lattice (aurora page, grain 0). Pitch 16px,
    // radius 2px, strength 0.5 (test drive; themes ship ~0.2): fragment
    // positions are pixel CENTRES, so fract(p/16)=0.5 puts dot centres at
    // px (7.5, 7.5) + 16k — pixel (8,8) sits 0.7px away (full mask), pixel
    // (0,0) sits ~10.6px away (zero mask).
    // -------------------------------------------------------
    let aurora = [15.0 / 255.0, 17.0 / 255.0, 23.0 / 255.0, 1.0];
    let dots = ModernPaper {
        color_a: [138.0 / 255.0, 180.0 / 255.0, 248.0 / 255.0],
        color_b: [197.0 / 255.0, 138.0 / 255.0, 249.0 / 255.0],
        dots: 0.5,
        spacing: [16.0, 16.0],
        radius: 2.0,
        // The lattice cases isolate the dots: no wash under them.
        wash: 0.0,
        phase: 0.0,
        focus: [0.5, 0.5],
        focus_pull: 0.0,
    };
    paper_bg.update_uniform(&queue, aurora, (64.0, 64.0), 1.0, 0.0, Some(&dots));
    let dot_pixels = render_64x64(&device, &queue, &paper_bg);
    let dp = |x: usize, y: usize| -> (i32, i32, i32) {
        let off = y * 256 + x * 4;
        (
            dot_pixels[off] as i32,
            dot_pixels[off + 1] as i32,
            dot_pixels[off + 2] as i32,
        )
    };
    let (dr, dg, db) = dp(8, 8);
    let (fr, fg, fb) = dp(0, 0);
    let (pr, _, _) = dp(24, 8); // one pitch to the right — periodicity
    eprintln!(
        "paperbg_headless [dots]: dot=({dr},{dg},{db}) flat=({fr},{fg},{fb}) next_dot_R={pr}"
    );
    // F1: a dot centre lifts hard toward the pole tint (strength 0.5 on a
    // near-black page ≈ half the tint's channel values).
    assert!(
        (60..=100).contains(&dr) && (60..=110).contains(&dg) && (100..=170).contains(&db),
        "F1 failed: dot centre ({dr},{dg},{db}) not in the tinted band"
    );
    // F2: between dots the page is untouched (grain 0 → vignette only).
    assert!(
        (12..=17).contains(&fr) && (14..=19).contains(&fg) && (20..=25).contains(&fb),
        "F2 failed: flat pixel ({fr},{fg},{fb}) should be the bare page"
    );
    // F3: the lattice repeats on the pitch.
    assert!(
        (dr - pr).abs() <= 6,
        "F3 failed: dot R={dr} vs next-pitch dot R={pr}"
    );
    // F5: the tint slides pole_a→pole_b along the diagonal: a dot near the
    // bottom-right corner (centre 55.5+0.5 ≈ 1.4px off, mask still ~1) is
    // measurably redder than the top-left one (pole_b.r > pole_a.r).
    let (br_r, _, _) = dp(56, 56);
    assert!(
        br_r - dr >= 10,
        "F5 failed: diagonal tint should rise, top-left R={dr} bottom-right R={br_r}"
    );
    // F4: dots off (None) leaves the same pixel flat — the lattice is opt-in.
    paper_bg.update_uniform(&queue, aurora, (64.0, 64.0), 1.0, 0.0, None);
    let off_pixels = render_64x64(&device, &queue, &paper_bg);
    let off_r = pixel_r(&off_pixels, 8, 8) as i32;
    assert!(
        (12..=17).contains(&off_r),
        "F4 failed: with dots None pixel (8,8) R={off_r} should be bare page"
    );

    // -------------------------------------------------------
    // Case 6: the gradient wash — two broad pools of pole light under the
    // page, rotating with the phase. Dots off so the pools are measured
    // alone; grain off so the reads are exact. On a square 64x64 surface
    // phase 0 puts pole A at the left edge (uv 0.05) and pole B at the
    // right (uv 0.95), both on the vertical midline.
    // -------------------------------------------------------
    let mut wash = ModernPaper {
        color_a: [138.0 / 255.0, 180.0 / 255.0, 248.0 / 255.0], // blue
        color_b: [249.0 / 255.0, 138.0 / 255.0, 160.0 / 255.0], // rose
        dots: 0.0,
        spacing: [16.0, 16.0],
        radius: 2.0,
        wash: 0.6,
        // Centred orbit: the pool geometry below is measured against the page
        // centre, so the focus cases at the end move it deliberately.
        phase: 0.0,
        focus: [0.5, 0.5],
        focus_pull: 0.0,
    };
    paper_bg.update_uniform(&queue, aurora, (64.0, 64.0), 1.0, 0.0, Some(&wash));
    let w0 = render_64x64(&device, &queue, &paper_bg);
    let wp = |buf: &[u8], x: usize, y: usize| -> (i32, i32, i32) {
        let off = y * 256 + x * 4;
        (buf[off] as i32, buf[off + 1] as i32, buf[off + 2] as i32)
    };
    let (lr, lg, lb) = wp(&w0, 3, 32); // pool A centre
    let (rr, rg, rb) = wp(&w0, 60, 32); // pool B centre
    let (cr, cg, cb) = wp(&w0, 32, 32); // between the pools
    let (tr, tg, tb) = wp(&w0, 32, 3); // off the pools' axis entirely
    eprintln!(
        "paperbg_headless [wash phase 0]: poleA=({lr},{lg},{lb}) poleB=({rr},{rg},{rb}) centre=({cr},{cg},{cb}) top=({tr},{tg},{tb})"
    );
    // W1: the pool centres carry their OWN pole, not a blend — pole A is the
    // blue one (b > r), pole B the rose one (r > b), each lifted far off the
    // near-black page.
    assert!(
        lb - lr >= 40 && lb >= 120,
        "W1 failed: pole A pool ({lr},{lg},{lb}) should read blue"
    );
    assert!(
        rr - rb >= 40 && rr >= 120,
        "W1 failed: pole B pool ({rr},{rg},{rb}) should read rose"
    );
    // W2: the wash falls off between the pools — the midpoint keeps a lift
    // (it is one page-wide gradient, not two spotlights) but a fraction of
    // either centre's, and a point off the axis is fainter still.
    let lift = |(r, g, b): (i32, i32, i32)| r + g + b - (15 + 17 + 23);
    let (l_lift, r_lift, c_lift, t_lift) = (
        lift((lr, lg, lb)),
        lift((rr, rg, rb)),
        lift((cr, cg, cb)),
        lift((tr, tg, tb)),
    );
    assert!(
        c_lift > 20 && c_lift * 2 < l_lift && c_lift * 2 < r_lift,
        "W2 failed: centre lift {c_lift} vs pools {l_lift}/{r_lift}"
    );
    assert!(
        t_lift < c_lift,
        "W2 failed: off-axis lift {t_lift} should be under the midpoint's {c_lift}"
    );
    // W3: a quarter turn swings the pools clockwise onto the vertical axis —
    // the left edge falls back toward the bare page and pole A's blue is now
    // at the TOP, as strong as it was at the left.
    wash.phase = 0.25;
    paper_bg.update_uniform(&queue, aurora, (64.0, 64.0), 1.0, 0.0, Some(&wash));
    let w25 = render_64x64(&device, &queue, &paper_bg);
    let (qlr, qlg, qlb) = wp(&w25, 3, 32);
    let (tmr, tmg, tmb) = wp(&w25, 32, 3);
    eprintln!(
        "paperbg_headless [wash phase 0.25]: left=({qlr},{qlg},{qlb}) top=({tmr},{tmg},{tmb})"
    );
    assert!(
        lift((qlr, qlg, qlb)) * 2 < l_lift,
        "W3 failed: left edge kept its pool after a quarter turn ({qlr},{qlg},{qlb})"
    );
    assert!(
        (tmb - lb).abs() <= 12 && tmb - tmr >= 40,
        "W3 failed: pole A should now sit at the top, got ({tmr},{tmg},{tmb}) vs left-at-0 ({lr},{lg},{lb})"
    );
    // W4: the strength is a real dial, not a switch — halving it halves the
    // lift at the pool's centre (the mix is linear in `wash`).
    wash.phase = 0.0;
    wash.wash = 0.3;
    paper_bg.update_uniform(&queue, aurora, (64.0, 64.0), 1.0, 0.0, Some(&wash));
    let whalf = render_64x64(&device, &queue, &paper_bg);
    let half_lift = lift(wp(&whalf, 3, 32));
    eprintln!("paperbg_headless [wash 0.3]: pool lift {half_lift} vs {l_lift} at 0.6");
    assert!(
        (half_lift * 2 - l_lift).abs() <= 12,
        "W4 failed: half the strength should be half the lift, got {half_lift} vs {l_lift}/2"
    );

    // W5: a pool is ROUND, not a smear across the page — on a 4:1 window the
    // coordinates are aspect-corrected, so pole A's light is spent well
    // before the middle of the row it sits on. (Without the correction the
    // pool stretches with the window and this pixel is still lit.)
    wash.wash = 0.6;
    paper_bg.update_uniform(&queue, aurora, (128.0, 32.0), 1.0, 0.0, Some(&wash));
    let wide = render_offscreen(&device, &queue, &paper_bg, 128, 32);
    let wq = |x: usize, y: usize| -> (i32, i32, i32) {
        let off = y * 128 * 4 + x * 4;
        (wide[off] as i32, wide[off + 1] as i32, wide[off + 2] as i32)
    };
    let pool = wq(6, 16); // pole A's centre: uv.x 0.05 either way
    let quarter = wq(48, 16); // a third of the way in, same row
    eprintln!("paperbg_headless [wash 4:1]: pool={pool:?} quarter_in={quarter:?}");
    assert!(
        lift(pool) > 200,
        "W5 failed: the pool itself should still be lit on a wide page, got {pool:?}"
    );
    assert!(
        lift(quarter) < 20,
        "W5 failed: a round pool must not smear across the row, got {quarter:?}"
    );

    // -------------------------------------------------------
    // Case 7: the focus pull — the whole orbit slides toward the focused
    // card, so the page's light gathers where the work is. Square surface,
    // phase 0 (pole A left, pole B right), pull toward the TOP of the page.
    // -------------------------------------------------------
    wash.wash = 0.6;
    wash.focus = [0.5, 0.0]; // a card at the top edge
    wash.focus_pull = 0.55; // washfocus::PULL
    paper_bg.update_uniform(&queue, aurora, (64.0, 64.0), 1.0, 0.0, Some(&wash));
    let wf = render_64x64(&device, &queue, &paper_bg);
    let top_row = lift(wp(&wf, 3, 6));
    let bot_row = lift(wp(&wf, 3, 58));
    let base_top = lift(wp(&w0, 3, 6));
    let base_bot = lift(wp(&w0, 3, 58));
    eprintln!(
        "paperbg_headless [focus top]: top {base_top}->{top_row} bottom {base_bot}->{bot_row}"
    );
    // F6: with the orbit centred the page is symmetric top-to-bottom; pulled
    // to the top it is not, and the asymmetry points the right way.
    assert!(
        (base_top - base_bot).abs() <= 8,
        "F6 premise: a centred wash should be symmetric, {base_top} vs {base_bot}"
    );
    assert!(
        top_row - bot_row > 60,
        "F6 failed: the light should gather at the top, {top_row} vs {bot_row}"
    );
    // F7: pull 0 is the centred page exactly — the pre-focus behaviour, and
    // what a frame with nothing focused draws.
    wash.focus_pull = 0.0;
    paper_bg.update_uniform(&queue, aurora, (64.0, 64.0), 1.0, 0.0, Some(&wash));
    let wf0 = render_64x64(&device, &queue, &paper_bg);
    assert_eq!(
        wf0, w0,
        "F7 failed: pull 0 must render the centred wash byte for byte"
    );

    // W6: strength 0 is a true off — the same pixels are the bare page, so
    // non-modern themes (and a wash-less modern theme) pay nothing.
    wash.wash = 0.0;
    paper_bg.update_uniform(&queue, aurora, (64.0, 64.0), 1.0, 0.0, Some(&wash));
    let woff = render_64x64(&device, &queue, &paper_bg);
    let (zr, zg, zb) = wp(&woff, 3, 32);
    assert!(
        (12..=17).contains(&zr) && (14..=19).contains(&zg) && (20..=25).contains(&zb),
        "W6 failed: wash 0 pixel ({zr},{zg},{zb}) should be the bare page"
    );
}
