//! Headless GPU test for the backdrop's GLOW: the soft light at the middle
//! of an awake page that beats and radiates — that it lights the lattice,
//! that it stays at the page's middle, that it beats, that each beat sends a
//! ring outward, that it has no seam, that it runs from one pole to the
//! other, and that a still page wears none of it. Each is a pure function of
//! the clocks and the wake the app hands the pass, so each is shot at chosen
//! values and read back. Skips on a GPU-less machine (CI) instead of failing.
mod common;

use common::backdrop::*;
use common::render_offscreen;
use crew_render::{ModernPaper, PaperBgPass, WashClocks};

/// Beats per orbit (the shader's `BEATS`): phase `t / BEATS` is `t` of the
/// way through a beat.
const BEATS: f32 = 4.0;

#[test]
fn backdrop_glow_headless() {
    let Some((device, queue)) = common::gpu() else {
        eprintln!("backdrop_glow_headless: no GPU adapter, skipping");
        return;
    };
    let pass = PaperBgPass::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let big = |m: &ModernPaper| {
        pass.update_uniform(&queue, DARK, (128.0, 128.0), 1.0, 0.0, Some(m));
        render_offscreen(&device, &queue, &pass, 128, 128)
    };
    // A lattice fine enough to read as a field (every pixel is the same
    // distance from its dot) and faint enough that the core never saturates.
    let fine = |a, b, clocks: WashClocks| ModernPaper {
        spacing: [2.0, 2.0],
        radius: 0.9,
        dots: 0.1,
        clocks: WashClocks {
            live: 1.0,
            ..clocks
        },
        ..lattice(a, b, 0.0)
    };
    let beat = |t: f32, eddy| WashClocks {
        phase: t / BEATS,
        eddy,
        ..Default::default()
    };
    let at = |buf: &[u8], x: usize, y: usize| lift(DARK, rgb_w(buf, 128, x, y));

    // L1: the glow lights the lattice. One pole, so the turning tint cannot
    // move a pixel. Asleep the lattice is the same everywhere; awake, at the
    // top of a beat, the middle carries several times the rim's light.
    let asleep = big(&with(fine(BLUE, BLUE, beat(0.0, 0.0)), |k| k.live = 0.0));
    let awoke = big(&fine(BLUE, BLUE, beat(0.0, 0.0)));
    let (a_mid, a_rim) = (at(&asleep, 64, 64), at(&asleep, 4, 64));
    let (mid, rim) = (at(&awoke, 64, 64), at(&awoke, 4, 64));
    eprintln!("[glow] middle/rim {a_mid}/{a_rim} asleep -> {mid}/{rim} awake");
    assert!(
        (a_mid - a_rim).abs() <= 3,
        "L1 premise: asleep, the lattice is uniform"
    );
    assert!(
        mid >= 3 * rim && mid - rim >= 60,
        "L1 failed: the middle should glow, {mid} vs {rim}"
    );

    // C1: it stays at the PAGE's middle. Through a beat, with the eddies
    // stirred and the pools' orbit pulled hard toward a corner, the brightest
    // point of the lattice is within a few pixels of the middle.
    for (t, eddy) in [(0.0, 0.0), (0.1, 0.4), (0.6, 0.8), (0.9, 0.2)] {
        let m = ModernPaper {
            focus: [0.15, 0.2],
            focus_pull: 1.0,
            ..fine(BLUE, BLUE, beat(t, eddy))
        };
        let buf = big(&m);
        let (x, y) = (0..128 * 128)
            .map(|i| (i % 128, i / 128))
            .max_by_key(|&(x, y)| {
                at(&buf, x, y) * 1000 - ((x as i32 - 64).pow(2) + (y as i32 - 64).pow(2))
            })
            .unwrap();
        eprintln!("[centre] beat {t}, eddy {eddy}: brightest at ({x}, {y})");
        assert!(
            x.abs_diff(64) <= 4 && y.abs_diff(64) <= 4,
            "C1 failed: the glow should stay in the middle, brightest at ({x}, {y})"
        );
    }

    // B1: it beats. The core is brightest at the top of a beat and at its
    // dimmest half a beat later.
    let core = |t| at(&big(&fine(BLUE, BLUE, beat(t, 0.0))), 64, 64);
    let (top, low) = (core(0.0), core(0.5));
    eprintln!("[beat] core {top} at the top, {low} half a beat on");
    assert!(
        top * 10 >= low * 13,
        "B1 failed: the core should beat, {top} vs {low}"
    );

    // R1: and each beat radiates. Out along the row to the right, past the
    // core, the ring is where the page most outshines the same page between
    // rings (at the very end of a beat); a fifth of a beat later it is
    // clearly further out.
    let rest = big(&fine(BLUE, BLUE, beat(0.999, 0.0)));
    let ring_at = |t| {
        let buf = big(&fine(BLUE, BLUE, beat(t, 0.0)));
        (20..63)
            .max_by_key(|&d| at(&buf, 64 + d, 64) - at(&rest, 64 + d, 64))
            .unwrap()
    };
    let (early, late) = (ring_at(0.2), ring_at(0.4));
    eprintln!("[ring] {early}px out a fifth into the beat, {late}px at two fifths");
    assert!(
        late >= early + 15 && late < 62,
        "R1 failed: the ring should travel outward, {early} -> {late}px"
    );

    // G1: no seam. Across the orbit's wrap and the eddies', each sample
    // moves about as far as the same-sized step after it.
    let row = |buf: &[u8]| -> Vec<i32> { (0..16).map(|i| at(buf, 4 + 8 * i, 64)).collect() };
    for name in ["orbit", "eddy"] {
        let clock = |t: f32| match name {
            "orbit" => beat(t * BEATS, 0.3),
            _ => beat(0.3, t),
        };
        let shot = |t: f32| row(&big(&fine(BLUE, BLUE, clock(t))));
        let (before, top, after) = (shot(0.999), shot(0.0), shot(0.001));
        for i in 0..16 {
            let (seam, step) = ((top[i] - before[i]).abs(), (after[i] - top[i]).abs());
            assert!(
                seam <= 2 * step + 3,
                "G1 failed: sample {i} jumps {seam} at the {name} clock's wrap, {step} a step later"
            );
        }
    }

    // H1: pole A at the core, pole B toward the rim. Measured square to the
    // lattice tint's axis at phase 0.875 — the tint alone changes nothing
    // down the middle column — just under the core and a ring's width out.
    let two = big(&ModernPaper {
        dots: 0.3,
        ..fine(
            BLUE,
            ROSE,
            WashClocks {
                phase: 0.875,
                ..Default::default()
            },
        )
    });
    let blue = |y| {
        let (r, _, b) = rgb_w(&two, 128, 64, y);
        b - r
    };
    let (inner, outer) = (blue(66), blue(110));
    eprintln!("[colour] blueness {inner} at the core, {outer} toward the rim");
    assert!(
        inner - outer >= 25,
        "H1 failed: the core should lean to pole A and the rim to pole B, {inner} vs {outer}"
    );
}
