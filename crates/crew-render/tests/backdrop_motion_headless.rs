//! Headless GPU test for the backdrop's MOTION: the pool breath, the pools'
//! wander, the lattice's turning tint and its glint. Each is a pure function
//! of the two clocks the app hands the pass, so each is shot at chosen phases
//! and read back. Skips on a GPU-less machine (CI) instead of failing.
mod common;

use common::backdrop::*;
use common::render_offscreen;
use crew_render::{ModernPaper, PaperBgPass};

#[test]
fn backdrop_motion_headless() {
    let Some((device, queue)) = common::gpu() else {
        eprintln!("backdrop_motion_headless: no GPU adapter, skipping");
        return;
    };
    let pass = PaperBgPass::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let shot = |page: [f32; 4], m: &ModernPaper| {
        pass.update_uniform(&queue, page, (64.0, 64.0), 1.0, 0.0, Some(m));
        render_offscreen(&device, &queue, &pass, 64, 64)
    };

    // B1: the pools breathe in counter-phase. An eighth of a turn is the top
    // of pool A's swell (breath = sin 2θ = 1) and three eighths its ebb; on a
    // square page pool A's centre is then at uv (0.18, 0.18) and (0.82, 0.18)
    // — pixels (11, 11) and (52, 11) — and pool B's opposite it.
    let swell = shot(DARK, &wash(0.125, 0.0));
    let ebb = shot(DARK, &wash(0.375, 0.0));
    let (a_swell, a_ebb) = (rgb(&swell, 11, 11), rgb(&ebb, 52, 11));
    let (b_ebb, b_swell) = (rgb(&swell, 52, 52), rgb(&ebb, 11, 52));
    eprintln!("[breath] A {a_swell:?} -> {a_ebb:?}, B {b_ebb:?} -> {b_swell:?}");
    assert!(
        a_swell.2 - a_ebb.2 >= 40,
        "B1 failed: pool A should be brighter at its swell, {a_swell:?} vs {a_ebb:?}"
    );
    assert!(
        b_swell.0 - b_ebb.0 >= 40,
        "B1 failed: pool B should dim while A swells, {b_ebb:?} vs {b_swell:?}"
    );

    // B2: at a quarter turn the breath is between swells, so the pool sits
    // at the strength it has at rest — the light moves between the poles
    // rather than the page pulsing as a whole.
    let rest = shot(DARK, &wash(0.0, 0.0));
    let quarter = shot(DARK, &wash(0.25, 0.0));
    let (at_rest, at_quarter) = (rgb(&rest, 3, 32), rgb(&quarter, 32, 3));
    assert!(
        (at_rest.2 - at_quarter.2).abs() <= 6,
        "B2 failed: pool A at a quarter turn {at_quarter:?} vs at rest {at_rest:?}"
    );

    // W1: with no wander the page is symmetric top to bottom (phase 0 puts
    // the pools on the horizontal midline). A twelfth of the wander clock is
    // the crest of the lean (sin 3w = 1): both pools tip toward the TOP, so
    // the light gathers there and the bottom goes dark.
    let (top0, bot0) = (
        lift(DARK, rgb(&rest, 32, 12)),
        lift(DARK, rgb(&rest, 32, 52)),
    );
    let leaned = shot(DARK, &wash(0.0, 1.0 / 12.0));
    let (top, bot) = (
        lift(DARK, rgb(&leaned, 32, 12)),
        lift(DARK, rgb(&leaned, 32, 52)),
    );
    eprintln!("[wander] top/bottom {top0}/{bot0} -> {top}/{bot}");
    assert!(
        (top0 - bot0).abs() <= 4,
        "W1 premise: unwandered page should be symmetric, {top0} vs {bot0}"
    );
    assert!(
        top - bot >= 30,
        "W1 failed: the leaning pools should light the top, {top} vs {bot}"
    );

    // T1: the lattice's tint turns with the orbit. At rest pole A is the
    // top-left end (bluer: lower R), as it always was; half a turn later the
    // axis has swung round and pole A is the bottom-right end.
    let t0 = shot(DARK, &lattice(BLUE, VIOLET, 0.0));
    let t5 = shot(DARK, &lattice(BLUE, VIOLET, 0.5));
    let (tl0, br0) = (rgb(&t0, 8, 8).0, rgb(&t0, 56, 56).0);
    let (tl5, br5) = (rgb(&t5, 8, 8).0, rgb(&t5, 56, 56).0);
    eprintln!("[tint] R top-left/bottom-right {tl0}/{br0} -> {tl5}/{br5}");
    assert!(br0 - tl0 >= 10, "T1 premise: rest tint {tl0} -> {br0}");
    assert!(
        tl5 - br5 >= 10,
        "T1 failed: half a turn should reverse it, {tl5} -> {br5}"
    );

    // G1: the glint. One pole, so the turning tint cannot move a pixel and
    // anything that changes is the band. At rest the lattice is uniform (the
    // band is parked off the page); at a quarter turn the crest lies across
    // the page's middle diagonal, where dot (24, 40) sits, and that dot
    // carries far more of the tint — while dot (8, 8), out of the band's
    // reach, is untouched.
    let g0 = shot(DARK, &lattice(BLUE, BLUE, 0.0));
    let g25 = shot(DARK, &lattice(BLUE, BLUE, 0.25));
    let (mid0, mid25) = (lift(DARK, rgb(&g0, 24, 40)), lift(DARK, rgb(&g25, 24, 40)));
    let (far0, far25) = (lift(DARK, rgb(&g0, 8, 8)), lift(DARK, rgb(&g25, 8, 8)));
    eprintln!("[glint] crest dot {mid0} -> {mid25}, far dot {far0} -> {far25}");
    assert!(
        (mid0 - far0).abs() <= 3,
        "G1 premise: a resting lattice should be uniform, {mid0} vs {far0}"
    );
    assert!(
        mid25 * 10 >= mid0 * 18,
        "G1 failed: the crest should lift its dot hard, {mid0} -> {mid25}"
    );
    assert!(
        (far25 - far0).abs() <= 3,
        "G1 failed: the band must stay local, far dot {far0} -> {far25}"
    );

    // G2: the wrap is never seen. Just before the sweep restarts, the crest
    // is past the far corner and the page is the resting lattice again.
    let wrap = shot(DARK, &lattice(BLUE, BLUE, 0.499));
    for (x, y) in [(8, 8), (24, 40), (56, 56)] {
        let (r0, rw) = (lift(DARK, rgb(&g0, x, y)), lift(DARK, rgb(&wrap, x, y)));
        assert!(
            (r0 - rw).abs() <= 3,
            "G2 failed: dot ({x},{y}) glints at the wrap, {r0} vs {rw}"
        );
    }

    // L1: it all reads on a LIGHT page too, at the strengths the light
    // themes ship (wash 0.12, dots 0.16): the breath moves pool A's colour
    // and the glint darkens the crest's dots by a visible step. Measured as
    // the change in the pixel itself — on a light page a pole can sit on
    // either side of the paper per channel, so "lift" would half-cancel.
    let light = |phase| ModernPaper {
        wash: 0.12,
        ..wash(phase, 0.0)
    };
    let (ls, le) = (shot(LIGHT, &light(0.125)), shot(LIGHT, &light(0.375)));
    let breath = dist(rgb(&ls, 11, 11), rgb(&le, 52, 11));
    let dots = |phase| ModernPaper {
        dots: 0.16,
        ..lattice(BLUE, BLUE, phase)
    };
    let (lg0, lg25) = (shot(LIGHT, &dots(0.0)), shot(LIGHT, &dots(0.25)));
    let glint = dist(rgb(&lg0, 24, 40), rgb(&lg25, 24, 40));
    eprintln!("[light] breath moves pool A by {breath}, glint moves its dot by {glint}");
    assert!(
        breath >= 6,
        "L1 failed: the breath should show on a light page, moved {breath}"
    );
    assert!(
        glint >= 25,
        "L1 failed: the glint should show on a light page, moved {glint}"
    );
}
