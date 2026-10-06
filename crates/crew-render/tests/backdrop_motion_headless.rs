//! Headless GPU test for the backdrop's MOTION: the pool breath, the pools'
//! wander, the page's whirlpool, the lattice's turning tint and the vortex.
//! Each is a pure function of the two clocks and the wake the app hands the
//! pass, so each is shot at chosen values and read back. Skips on a GPU-less
//! machine (CI) instead of failing.
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

    // V1: the whirlpool. Awake, and half way round the slow clock, the lean
    // and reach are both at zero, so the pools sit where they rest (A blue
    // at the left, B rose at the right) and only the whirl has moved
    // anything. On a ring round the centre pool A shows as the
    // bluest angle; at rest every ring finds it due left, and in the whirl
    // the inner ring finds it turned FURTHER than the outer one — the line
    // between the pools has wound into a spiral, not swung as a bar.
    let big = |m: &ModernPaper| {
        pass.update_uniform(&queue, DARK, (128.0, 128.0), 1.0, 0.0, Some(m));
        render_offscreen(&device, &queue, &pass, 128, 128)
    };
    let (still, whirl) = (big(&wash(0.0, 0.0)), big(&awake(wash(0.0, 0.5))));
    let (inner0, outer0) = (bluest_turn(&still, 0.15), bluest_turn(&still, 0.4));
    let (inner, outer) = (bluest_turn(&whirl, 0.15), bluest_turn(&whirl, 0.4));
    eprintln!("[whirl] inner/outer turn {inner0}/{outer0} -> {inner}/{outer} deg");
    assert!(
        inner0.abs() <= 5 && outer0.abs() <= 5,
        "V1 premise: a still page finds pool A due left, {inner0}/{outer0}"
    );
    assert!(
        inner.signum() == outer.signum() && outer.abs() >= 15,
        "V1 failed: the whirl should turn the whole page one way, {inner}/{outer}"
    );
    assert!(
        inner.abs() - outer.abs() >= 20,
        "V1 failed: the centre should turn further than the rim, {inner}/{outer}"
    );

    // V2: the flow never stops, so it has no seam to hide: awake, the page
    // just before either clock comes round is the page at the top of it.
    let top = shot(DARK, &awake(wash(0.0, 0.0)));
    let (wrap_w, wrap_p) = (
        shot(DARK, &awake(wash(0.0, 0.999))),
        shot(DARK, &awake(wash(0.999, 0.0))),
    );
    for (x, y) in [(12, 20), (32, 32), (50, 44), (20, 50)] {
        let (dw, dp) = (
            dist(rgb(&top, x, y), rgb(&wrap_w, x, y)),
            dist(rgb(&top, x, y), rgb(&wrap_p, x, y)),
        );
        assert!(
            dw <= 6 && dp <= 6,
            "V2 failed: ({x},{y}) jumps {dw}/{dp} at the wraps"
        );
    }

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

    // G1: the vortex. One pole, so the turning tint cannot move a pixel and
    // anything that changes is the bands. Asleep the lattice is uniform;
    // awake, the dots rise and fall band by band, the crest's carrying far
    // more of the tint than a trough's.
    let dots16 = |buf: &[u8]| -> Vec<i32> {
        (0..16)
            .map(|i| lift(DARK, rgb(buf, 8 + 16 * (i % 4), 8 + 16 * (i / 4))))
            .collect()
    };
    let (asleep, vortex) = (
        dots16(&shot(DARK, &lattice(BLUE, BLUE, 0.0))),
        dots16(&shot(DARK, &awake(lattice(BLUE, BLUE, 0.0)))),
    );
    let span = |v: &[i32]| (*v.iter().min().unwrap(), *v.iter().max().unwrap());
    let ((a_lo, a_hi), (v_lo, v_hi)) = (span(&asleep), span(&vortex));
    eprintln!("[vortex] asleep dots {a_lo}..{a_hi}, awake {v_lo}..{v_hi}");
    assert!(
        a_hi - a_lo <= 3,
        "G1 premise: a sleeping lattice is uniform, {asleep:?}"
    );
    assert!(
        v_hi * 10 >= v_lo * 25,
        "G1 failed: the bands should lift their dots hard, {vortex:?}"
    );

    // G2: no seam. The step across the orbit's wrap (0.999 -> 0) moves each
    // dot about as far as the same-sized step after it (0 -> 0.001) — the
    // crest's slope differs a little either side, a seam would be a jump of
    // hundreds: the bands just keep pouring.
    let before = dots16(&shot(DARK, &awake(lattice(BLUE, BLUE, 0.999))));
    let after = dots16(&shot(DARK, &awake(lattice(BLUE, BLUE, 0.001))));
    for i in 0..16 {
        let (seam, step) = ((vortex[i] - before[i]).abs(), (after[i] - vortex[i]).abs());
        assert!(
            seam <= 2 * step + 3,
            "G2 failed: dot {i} jumps {seam} at the wrap, {step} a step later"
        );
    }

    // L1: it all reads on a LIGHT page too, at the strengths the light
    // themes ship (wash 0.12, dots 0.16): the breath moves pool A's colour
    // and a vortex band darkens its dots by a visible step. Measured as
    // the change in the pixel itself — on a light page a pole can sit on
    // either side of the paper per channel, so "lift" would half-cancel.
    let light = |phase| ModernPaper {
        wash: 0.12,
        ..wash(phase, 0.0)
    };
    let (ls, le) = (shot(LIGHT, &light(0.125)), shot(LIGHT, &light(0.375)));
    let breath = dist(rgb(&ls, 11, 11), rgb(&le, 52, 11));
    let dots = |live| ModernPaper {
        dots: 0.16,
        live,
        ..lattice(BLUE, BLUE, 0.0)
    };
    let (lg0, lg1) = (shot(LIGHT, &dots(0.0)), shot(LIGHT, &dots(1.0)));
    let glint = (0..16)
        .map(|i| (8 + 16 * (i % 4), 8 + 16 * (i / 4)))
        .map(|(x, y)| dist(rgb(&lg0, x, y), rgb(&lg1, x, y)))
        .max()
        .unwrap();
    eprintln!("[light] breath moves pool A by {breath}, a band moves its dot by {glint}");
    assert!(
        breath >= 6,
        "L1 failed: the breath should show on a light page, moved {breath}"
    );
    assert!(
        glint >= 25,
        "L1 failed: the glint should show on a light page, moved {glint}"
    );

    // G3: the bands pour INWARD. On a lattice fine enough to be a field,
    // along the row through the centre and out to the right, find the band
    // crest nearest a third of the way out; a third of a band's pour later
    // the nearest crest to it has moved toward the centre.
    let fine = |phase| ModernPaper {
        spacing: [2.0, 2.0],
        radius: 0.9,
        ..awake(lattice(BLUE, BLUE, phase))
    };
    let row = |buf: &[u8]| -> Vec<i32> {
        (0..128)
            .map(|x| lift(DARK, rgb_w(buf, 128, x, 64)))
            .collect()
    };
    let crest_near = |v: &[i32], at: usize| {
        (70..124)
            .filter(|&x| v[x] >= v[x - 1] && v[x] >= v[x + 1] && v[x] > v[x - 3] && v[x] > v[x + 3])
            .min_by_key(|&x| x.abs_diff(at))
            .unwrap()
    };
    let early = crest_near(&row(&big(&fine(0.1))), 64 + 26);
    let late = crest_near(&row(&big(&fine(0.1 + 1.0 / 18.0))), early);
    eprintln!("[pour] crest x {early} -> {late}");
    assert!(
        late + 2 <= early,
        "G3 failed: the band should sink inward, crest x {early} -> {late}"
    );
}

/// Which way the blue lies on a ring of `r` half-heights round the centre of
/// a 128px shot, in degrees (signed) turned from due left — where pool A, the
/// blue pole, rests. The ring's blueness (blue less red) summed as vectors:
/// its first harmonic, so a flat stretch of ring cannot pick an arbitrary
/// winner the way an argmax would.
fn bluest_turn(buf: &[u8], r: f32) -> i32 {
    let (mut sx, mut sy) = (0.0f32, 0.0f32);
    for deg in (0..360).step_by(3) {
        let a = (deg as f32).to_radians();
        let x = (64.0 + r * 128.0 * a.cos()) as usize;
        let y = (64.0 + r * 128.0 * a.sin()) as usize;
        let off = y * 512 + x * 4;
        let blue = (buf[off + 2] as i32 - buf[off] as i32) as f32;
        (sx, sy) = (sx + blue * a.cos(), sy + blue * a.sin());
    }
    let turn = sy.atan2(sx).to_degrees() - 180.0;
    (turn + 540.0).rem_euclid(360.0) as i32 - 180
}
