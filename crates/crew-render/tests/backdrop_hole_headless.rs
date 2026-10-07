//! Headless GPU test for the backdrop's BLACK HOLE (every theme but the
//! tubes): that a still page is untouched by it, that its shadow is dark and
//! its disk lit — straight across the middle, lensed over the top and under
//! the bottom — that it stays in the middle whatever the focus does, that
//! the disk turns, that a line sent drops a flare through it, and that on a
//! light page the shadow stays a dusk. Each is the awake shot against the
//! same page asleep, so the pools moving on their clocks cannot pass for
//! the hole. Skips on a GPU-less machine (CI) instead of failing.
mod common;

use common::backdrop::*;
use common::render_offscreen;
use crew_render::{ModernPaper, PaperBgPass};

const N: usize = 256;
/// Px per unit of the hole's mass on a 256-px page (`HOLE_M` × 256).
const M_PX: f32 = 0.022 * N as f32;

/// A Nebula-strength page: a quiet wash, no lattice, so the hole is read alone.
fn page(phase: f32) -> ModernPaper {
    ModernPaper {
        color_a: VIOLET,
        color_b: ROSE,
        wash: 0.15,
        ..wash(phase, 0.0)
    }
}

fn sum((r, g, b): (i32, i32, i32)) -> i32 {
    r + g + b
}

/// The pixel `m` masses right of the middle and `up` masses above it.
fn at(m: f32, up: f32) -> (usize, usize) {
    let c = N as f32 / 2.0;
    ((c + m * M_PX) as usize, (c - up * M_PX) as usize)
}

#[test]
fn backdrop_hole_headless() {
    let Some((device, queue)) = common::gpu() else {
        eprintln!("backdrop_hole_headless: no GPU adapter, skipping");
        return;
    };
    let pass = PaperBgPass::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let shot_on = |bg: [f32; 4], m: &ModernPaper, hole: bool| {
        pass.set_black_hole(hole);
        pass.update_uniform(&queue, bg, (N as f32, N as f32), 1.0, 0.0, Some(m));
        render_offscreen(&device, &queue, &pass, N as u32, N as u32)
    };
    let shot = |m: &ModernPaper| shot_on(DARK, m, true);
    let px = |buf: &[u8], (x, y): (usize, usize)| rgb_w(buf, N, x, y);
    // How much the hole adds at `p`: the awake shot against the same page
    // asleep, signed by brightness.
    let gain = |awake: &[u8], asleep: &[u8], p| sum(px(awake, p)) - sum(px(asleep, p));

    // K1: a still page wears no hole — the same bytes as the vortex's still
    // page, lattice and all, so every resting shot is untouched.
    let still = ModernPaper {
        dots: 0.3,
        ..page(0.2)
    };
    assert!(
        shot_on(DARK, &still, true) == shot_on(DARK, &still, false),
        "a still page is identical with the hole on"
    );

    let asleep = shot(&page(0.2));
    let awake_shot = shot(&awake(page(0.2)));

    // K2: the shadow. Inside it nothing gets out: darker than the page.
    let core = gain(&awake_shot, &asleep, at(0.0, 2.5));
    assert!(core < -15, "the shadow darkens the page: {core}");

    // K3: the disk straight across the middle, lit on both sides.
    for side in [-1.0, 1.0] {
        let lit = gain(&awake_shot, &asleep, at(side * 9.0, 0.0));
        assert!(lit > 60, "the disk at {side} is lit: {lit}");
    }

    // K4: lensing. The far side of the disk is lifted into an arch over the
    // shadow, and its underside bent round below it — light where a flat
    // disk would have none.
    let over = gain(&awake_shot, &asleep, at(0.0, 7.0));
    let under = gain(&awake_shot, &asleep, at(0.0, -7.0));
    assert!(over > 40, "the far side arches over the top: {over}");
    assert!(under > 30, "and bends round under the bottom: {under}");
    // …and well above the arch, nothing: the hole has an edge.
    let clear = gain(&awake_shot, &asleep, at(0.0, 20.0)).abs();
    assert!(clear < 12, "the page past the arch is untouched: {clear}");

    // K5: the hole stays in the middle. The orbit's centre follows the
    // focused card; the hole does not.
    let pulled = |m: ModernPaper| ModernPaper {
        focus: [0.9, 0.9],
        focus_pull: 1.0,
        ..m
    };
    let (asleep_p, awake_p) = (shot(&pulled(page(0.2))), shot(&pulled(awake(page(0.2)))));
    assert!(
        gain(&awake_p, &asleep_p, at(0.0, 2.5)) < -15,
        "still dark in the middle"
    );
    assert!(
        gain(&awake_p, &asleep_p, at(9.0, 0.0)) > 60,
        "still lit across it"
    );

    // K6: the disk turns. Along its middle, the light at one moment is not
    // the light a moment later.
    let later = (shot(&page(0.32)), shot(&awake(page(0.32))));
    let moved: i32 = (0..24)
        .map(|i| at(7.0 + 0.4 * i as f32, 0.0))
        .map(|p| (gain(&awake_shot, &asleep, p) - gain(&later.1, &later.0, p)).abs())
        .max()
        .unwrap();
    assert!(moved > 8, "the disk's streaks move: {moved}");

    // K7: a line sent drops a flare through the disk. Half way down its fall
    // it is a ring of light part way in, brighter than the disk it crosses.
    let flare = shot(&with(awake(page(0.2)), |k| k.ping = 0.65));
    let lift_by: i32 = (0..40)
        .map(|i| at(6.0 + 0.3 * i as f32, 0.0))
        .map(|p| sum(px(&flare, p)) - sum(px(&awake_shot, p)))
        .max()
        .unwrap();
    assert!(
        lift_by > 40,
        "the flare lights the disk as it falls: {lift_by}"
    );

    // K8: on a light page the shadow is a dusk, not a hole punched in the
    // paper — dark ink is read across it.
    let light_page = shot_on(LIGHT, &page(0.2), true);
    let light_awake = shot_on(LIGHT, &awake(page(0.2)), true);
    let (bare, dusk) = (
        sum(px(&light_page, at(0.0, 2.5))),
        sum(px(&light_awake, at(0.0, 2.5))),
    );
    assert!(dusk < bare - 30, "the shadow still reads: {bare} -> {dusk}");
    assert!(
        dusk * 10 > bare * 7,
        "but keeps most of the paper's light: {bare} -> {dusk}"
    );
}
