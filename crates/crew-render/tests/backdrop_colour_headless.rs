//! Headless GPU test for the backdrop's COLOUR in motion: the pools trading
//! colour as they turn, and the glow's sheen lighting the page itself. Shot
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

    // S1: the sheen. Awake, the glow lights the page itself: down the middle
    // of the page it rises to the core and falls away. The pools vary down
    // it only slowly, so what is left after a moving average a quarter of
    // the page wide is the glow's RIPPLE — next to nothing on a sleeping
    // page. One pole, so the glow's colours cannot beat against them.
    let one = |m: ModernPaper| ModernPaper {
        color_b: m.color_a,
        ..m
    };
    let big = |page: [f32; 4], m: &ModernPaper| {
        pass.update_uniform(&queue, page, (128.0, 128.0), 1.0, 0.0, Some(m));
        render_offscreen(&device, &queue, &pass, 128, 128)
    };
    let (asleep, glow) = (
        ripple(&big(DARK, &one(wash(0.0, 0.0))), DARK),
        ripple(&big(DARK, &awake(one(wash(0.0, 0.0)))), DARK),
    );
    eprintln!("[sheen] the page's ripple {asleep:.1} asleep -> {glow:.1} awake");
    assert!(
        glow >= 12.0 && glow >= 3.0 * asleep,
        "S1 failed: the glow should light the page, {asleep:.1} -> {glow:.1}"
    );

    // S2: the sheen reads on a LIGHT page at the strength light themes ship
    // (wash 0.12) — faint, half a pool's strength, but a visible ripple.
    let light = ModernPaper {
        wash: 0.12,
        ..awake(one(wash(0.0, 0.0)))
    };
    let lv = ripple(&big(LIGHT, &light), LIGHT);
    let l0 = ripple(&big(LIGHT, &with(light, |k| k.live = 0.0)), LIGHT);
    eprintln!("[sheen light] the page's ripple {l0:.1} asleep -> {lv:.1} awake");
    assert!(
        lv >= 1.2 && lv >= 3.0 * l0,
        "S2 failed: the sheen should show on a light page, {l0:.1} -> {lv:.1}"
    );
}

/// How much the page ripples down the middle column of a 128px shot: the
/// RMS of what is left after a moving average 31px wide, in
/// summed-channel levels from the bare `page`.
fn ripple(buf: &[u8], page: [f32; 4]) -> f32 {
    let v: Vec<f32> = (0..128)
        .map(|y| lift(page, rgb_w(buf, 128, 64, y)) as f32)
        .collect();
    let smooth = |k: usize| v[k - 15..=k + 15].iter().sum::<f32>() / 31.0;
    let ss: f32 = (15..113).map(|k| (v[k] - smooth(k)).powi(2)).sum();
    (ss / 98.0).sqrt()
}
