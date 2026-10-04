//! Headless GPU test for the backdrop's COLOUR in motion: the pools trading
//! colour as they turn, and the glint's sheen lighting the page itself. Shot
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

    // S1: the sheen. At a quarter turn one of the glint's spiral arms
    // crosses the top-right diagonal through pixel (53, 10). The pools are
    // then at the top and bottom centres, untraded and between breaths, so
    // the wash is mirror-symmetric left to right — but the arms are not (they
    // are symmetric under a half TURN), and (10, 10), the mirror of (53, 10),
    // lies midway between them. Whatever separates the two is the sheen alone.
    let (on, off) = (rgb(&quarter, 53, 10), rgb(&quarter, 10, 10));
    eprintln!("[sheen] crest {on:?} vs mirror {off:?}");
    assert!(
        lift(DARK, on) - lift(DARK, off) >= 30,
        "S1 failed: the crest should light the page, {on:?} vs mirror {off:?}"
    );

    // S2: the sheen reads on a LIGHT page at the strength light themes ship
    // (wash 0.12) — faint, half a pool's strength, but a visible step.
    let light = ModernPaper {
        wash: 0.12,
        ..wash(0.25, 0.0)
    };
    let lq = shot(LIGHT, &light);
    let (lon, loff) = (rgb(&lq, 53, 10), rgb(&lq, 10, 10));
    eprintln!("[sheen light] crest {lon:?} vs mirror {loff:?}");
    assert!(
        dist(lon, loff) >= 6,
        "S2 failed: the sheen should show on a light page, {lon:?} vs {loff:?}"
    );
}
