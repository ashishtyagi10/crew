//! Headless GPU test for the backdrop's COLOUR in motion: the pools trading
//! colour as they turn, and the vortex's sheen lighting the page itself. Shot
//! at chosen phases and read back, like `backdrop_motion_headless`. Skips on a
//! GPU-less machine (CI) instead of failing.
mod common;

use common::backdrop::*;
use common::render_offscreen;
use crew_render::{ModernPaper, PaperBgPass};

#[test]
fn backdrop_colour_headless() {
    let Some((device, queue)) = common::gpu() else {
        eprintln!("backdrop_colour_headless: no GPU adapter, skipping");
        return;
    };
    let pass = PaperBgPass::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let shot = |page: [f32; 4], m: &ModernPaper| {
        pass.update_uniform(&queue, page, (64.0, 64.0), 1.0, 0.0, Some(m));
        render_offscreen(&device, &queue, &pass, 64, 64)
    };

    // C1: the pools trade colour. At rest pool A (left edge, pixel (3, 32))
    // is the blue pole and pool B (right, (60, 32)) the rose one. An eighth
    // of a turn is the peak of the trade: pool A, now at (11, 11), has leaned
    // toward rose and pool B, at (52, 52), toward blue. Measured as the
    // balance between the two channels the poles differ in, because the
    // breath changes each pool's strength at the same moment — and the
    // breath alone moves both balances the OTHER way (a stronger blue pool
    // is bluer), so only the trade can pass this.
    let rest = shot(DARK, &wash(0.0, 0.0));
    let eighth = shot(DARK, &wash(0.125, 0.0));
    let (a0, b0) = (rgb(&rest, 3, 32), rgb(&rest, 60, 32));
    let (a1, b1) = (rgb(&eighth, 11, 11), rgb(&eighth, 52, 52));
    eprintln!("[trade] A {a0:?} -> {a1:?}, B {b0:?} -> {b1:?}");
    assert!(
        (a1.0 - a1.2) - (a0.0 - a0.2) >= 20,
        "C1 failed: pool A should lean toward rose, {a0:?} -> {a1:?}"
    );
    assert!(
        (b1.2 - b1.0) - (b0.2 - b0.0) >= 20,
        "C1 failed: pool B should lean toward blue, {b0:?} -> {b1:?}"
    );

    // C2: and at every quarter each pool is its own pole again — a quarter
    // turn puts pool A at the top, (32, 3), as blue as it was at the left.
    let quarter = shot(DARK, &wash(0.25, 0.0));
    let aq = rgb(&quarter, 32, 3);
    assert!(
        dist(aq, a0) <= 9,
        "C2 failed: pool A at a quarter turn {aq:?} should be its rest colour {a0:?}"
    );

    // S1: the sheen. Awake, the vortex's six bands light the page itself:
    // round a ring about the centre the page ripples once a band. The pools
    // vary round a ring only slowly, so what is left after a moving average
    // one band wide (60°) is the bands' RIPPLE — next to nothing on a
    // sleeping page. One pole, so the bands' cycling colours cannot beat
    // against them.
    let one = |m: ModernPaper| ModernPaper {
        color_b: m.color_a,
        ..m
    };
    let big = |page: [f32; 4], m: &ModernPaper| {
        pass.update_uniform(&queue, page, (128.0, 128.0), 1.0, 0.0, Some(m));
        render_offscreen(&device, &queue, &pass, 128, 128)
    };
    let (asleep, vortex) = (
        ripple(&big(DARK, &one(wash(0.25, 0.0))), DARK),
        ripple(&big(DARK, &awake(one(wash(0.25, 0.0)))), DARK),
    );
    eprintln!("[sheen] ring's ripple {asleep:.1} asleep -> {vortex:.1} awake");
    assert!(
        vortex >= 12.0 && vortex >= 3.0 * asleep,
        "S1 failed: the bands should light the page, {asleep:.1} -> {vortex:.1}"
    );

    // S2: the sheen reads on a LIGHT page at the strength light themes ship
    // (wash 0.12) — faint, half a pool's strength, but a visible ripple.
    let light = ModernPaper {
        wash: 0.12,
        ..awake(one(wash(0.25, 0.0)))
    };
    let lv = ripple(&big(LIGHT, &light), LIGHT);
    let l0 = ripple(&big(LIGHT, &ModernPaper { live: 0.0, ..light }), LIGHT);
    eprintln!("[sheen light] ring's ripple {l0:.1} asleep -> {lv:.1} awake");
    assert!(
        lv >= 1.2 && lv >= 3.0 * l0,
        "S2 failed: the sheen should show on a light page, {l0:.1} -> {lv:.1}"
    );
}

/// How much the page ripples round a ring of 0.35 half-heights about the
/// centre of a 128px shot, sampled every 2°: the RMS of what is left after
/// a circular moving average 60° wide, in summed-channel levels from the
/// bare `page`.
fn ripple(buf: &[u8], page: [f32; 4]) -> f32 {
    let v: Vec<f32> = (0..180)
        .map(|k| {
            let a = (2.0 * k as f32).to_radians();
            let x = (64.0 + 0.35 * 64.0 * a.cos()) as usize;
            let y = (64.0 + 0.35 * 64.0 * a.sin()) as usize;
            lift(page, rgb_w(buf, 128, x, y)) as f32
        })
        .collect();
    let smooth = |k: usize| (0..30).map(|d| v[(k + 180 - 15 + d) % 180]).sum::<f32>() / 30.0;
    let ss: f32 = (0..180).map(|k| (v[k] - smooth(k)).powi(2)).sum();
    (ss / 180.0).sqrt()
}
