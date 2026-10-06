//! Headless GPU test for the backdrop's VORTEX: the spiral of soft bands that
//! pours inward on an awake page — how hard its bands light the lattice, that
//! it has no seam, that it pours inward, that each band wears its own colour,
//! that its winding breathes, that a counter-spiral beads its arms, that it
//! pulls like breath and that its bands melt into a ring of light at the eye. Each
//! is a pure function of the clocks and the wake the app hands the pass, so
//! each is shot at chosen values and read back. Skips on a GPU-less machine
//! (CI) instead of failing.
mod common;

use common::backdrop::*;
use common::render_offscreen;
use crew_render::{ModernPaper, PaperBgPass};

#[test]
fn backdrop_vortex_headless() {
    let Some((device, queue)) = common::gpu() else {
        eprintln!("backdrop_vortex_headless: no GPU adapter, skipping");
        return;
    };
    let pass = PaperBgPass::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let shot = |m: &ModernPaper| {
        pass.update_uniform(&queue, DARK, (64.0, 64.0), 1.0, 0.0, Some(m));
        render_offscreen(&device, &queue, &pass, 64, 64)
    };
    let big = |m: &ModernPaper| {
        pass.update_uniform(&queue, DARK, (128.0, 128.0), 1.0, 0.0, Some(m));
        render_offscreen(&device, &queue, &pass, 128, 128)
    };
    let huge = |m: &ModernPaper| {
        pass.update_uniform(&queue, DARK, (256.0, 256.0), 1.0, 0.0, Some(m));
        render_offscreen(&device, &queue, &pass, 256, 256)
    };
    // A lattice fine enough to read as a field: every pixel is the same
    // distance from its dot, so a band shows as a smooth ridge, not dots.
    let fine = |a, b, phase, wander| ModernPaper {
        spacing: [2.0, 2.0],
        radius: 0.9,
        wander,
        ..awake(lattice(a, b, phase))
    };

    // G1: the bands light the lattice. One pole, so the turning tint cannot
    // move a pixel and anything that changes is the bands. Asleep the
    // lattice is uniform; awake, at the top of the pull (a quarter turn), the
    // dots rise and fall band by band, the crest's carrying far more of the
    // tint than a trough's.
    let dots16 = |buf: &[u8]| -> Vec<i32> {
        (0..16)
            .map(|i| lift(DARK, rgb(buf, 8 + 16 * (i % 4), 8 + 16 * (i / 4))))
            .collect()
    };
    let (asleep, vortex) = (
        dots16(&shot(&lattice(BLUE, BLUE, 0.0))),
        dots16(&shot(&awake(lattice(BLUE, BLUE, 0.25)))),
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
    let top = dots16(&shot(&awake(lattice(BLUE, BLUE, 0.0))));
    let before = dots16(&shot(&awake(lattice(BLUE, BLUE, 0.999))));
    let after = dots16(&shot(&awake(lattice(BLUE, BLUE, 0.001))));
    for i in 0..16 {
        let (seam, step) = ((top[i] - before[i]).abs(), (after[i] - top[i]).abs());
        assert!(
            seam <= 2 * step + 3,
            "G2 failed: dot {i} jumps {seam} at the wrap, {step} a step later"
        );
    }

    // G3: the bands pour INWARD. Along the row through the centre and out to
    // the right, find the crest nearest a third of the way out; a third of a
    // band's pour later the nearest crest to it has moved toward the centre.
    let right = |buf: &[u8]| ray(buf, (1, 0), 63);
    let early = nearest(&crests(&right(&big(&fine(BLUE, BLUE, 0.1, 0.0)))), 26);
    let late = nearest(
        &crests(&right(&big(&fine(BLUE, BLUE, 0.1 + 1.0 / 18.0, 0.0)))),
        early,
    );
    eprintln!("[pour] crest {early} px out -> {late}");
    assert!(
        late + 2 <= early,
        "G3 failed: the band should sink inward, crest {early} px out -> {late}"
    );

    // H1: each band wears its own colour. Along the ray from the centre to
    // the top-right corner — square to the lattice tint's axis at phase 0,
    // so the tint alone is the same colour all the way out — a sleeping page
    // is one colour, and an awake one's crests run between the two poles.
    let diag = |buf: &[u8]| -> Vec<(i32, i32)> {
        (0..56)
            .map(|t| {
                let (r, _, b) = rgb_w(buf, 128, 64 + t, 64 - t);
                (lift(DARK, (r, 0, b)), b - r)
            })
            .collect()
    };
    let (still, awoke) = (
        diag(&big(&lattice_w(BLUE, ROSE))),
        diag(&big(&fine(BLUE, ROSE, 0.0, 0.0))),
    );
    let peaks: Vec<usize> = crests(&awoke.iter().map(|p| p.0).collect::<Vec<_>>())
        .into_iter()
        .filter(|&t| t >= 8)
        .collect();
    let blue_at = |v: &[(i32, i32)]| -> Vec<i32> { peaks.iter().map(|&t| v[t].1).collect() };
    let (s_blue, a_blue) = (blue_at(&still), blue_at(&awoke));
    let spread = |v: &[i32]| v.iter().max().unwrap() - v.iter().min().unwrap();
    eprintln!("[colour] crests at {peaks:?}: blueness {s_blue:?} asleep -> {a_blue:?} awake");
    assert!(
        peaks.len() >= 3,
        "H1 premise: the ray should cross several bands, {peaks:?}"
    );
    assert!(
        spread(&s_blue) <= 8,
        "H1 premise: one colour asleep, {s_blue:?}"
    );
    assert!(
        spread(&a_blue) >= 40,
        "H1 failed: the bands should wear different colours, {a_blue:?}"
    );

    // W2: the spiral breathes. An eighth of the way round the slow clock it
    // is wound tightest and three eighths loosest; out along the same ray
    // the tight spiral crosses clearly more bands.
    let count = |wander| crests(&right(&big(&fine(BLUE, BLUE, 0.0, wander)))).len();
    let (tight, loose) = (count(0.125), count(0.375));
    eprintln!("[breath] bands crossed: {tight} wound tight, {loose} loose");
    assert!(
        tight >= loose + 2,
        "W2 failed: the winding should breathe, {tight} vs {loose} bands"
    );

    // K1: the counter-spiral beads the arms. A ring about the centre crosses
    // every band at the same radius (some twice, where the current ripples
    // one across it), so without beads their crests would stand level; the
    // beads leave some crests well short of the brightest.
    // A faint lattice, so no crest saturates its dots and hides a bead.
    let ring = big(&ModernPaper {
        dots: 0.1,
        ..fine(BLUE, BLUE, 0.0, 0.0)
    });
    let round: Vec<i32> = (0..120)
        .map(|step| {
            let a = (3.0 * step as f32).to_radians();
            let (x, y) = (64.0 + 26.0 * a.cos(), 64.0 + 26.0 * a.sin());
            lift(DARK, rgb_w(&ring, 128, x as usize, y as usize))
        })
        .collect();
    let heights: Vec<i32> = ring_crests(&round).into_iter().map(|d| round[d]).collect();
    let (lo, hi) = span(&heights);
    eprintln!("[beads] crest heights round the ring {heights:?}");
    assert!(
        heights.len() >= 6 && lo * 100 <= hi * 85,
        "K1 failed: beads should make the crests uneven, {heights:?}"
    );

    // E1: the eye. On a 256px page the bands melt into a ring of light about
    // 0.11 half-heights (14px) out, round a dark centre. The current rocks
    // the ring a few pixels, so each of 36 rays takes its brightest point
    // between 8 and 22px out; the centre is the mean within 3px. A sleeping
    // page is the same lattice everywhere.
    let faint = |phase| ModernPaper {
        dots: 0.1,
        ..fine(BLUE, BLUE, phase, 0.0)
    };
    let (ring0, mid0) = eye(&huge(&ModernPaper {
        live: 0.0,
        ..faint(0.25)
    }));
    let (ring, mid) = eye(&huge(&faint(0.25)));
    eprintln!("[eye] ring/centre {ring0}/{mid0} asleep -> {ring}/{mid} awake");
    assert!(
        ring0 - mid0 <= 6,
        "E1 premise: no ring asleep, {ring0} vs {mid0}"
    );
    assert!(
        ring >= mid * 2 && ring - mid >= 60,
        "E1 failed: the eye should be a ring of light round a dark centre, {ring} vs {mid}"
    );

    // P1: the eye brightens on the inhale. A quarter turn is the top of the
    // pull, the turn's top the bottom of it.
    let (low, _) = eye(&huge(&faint(0.0)));
    eprintln!("[pull] eye ring {low} exhaled -> {ring} inhaled");
    assert!(
        ring * 10 >= low * 13,
        "P1 failed: the inhale should brighten the eye, {low} -> {ring}"
    );

    // P2: and the pour quickens on the way in. The same small step of the
    // orbit carries a band crest further inward around an eighth of a turn
    // (the surge's peak) than around three eighths (its trough).
    let pour = |at: f32| {
        let step = 1.0 / 18.0;
        let from = nearest(
            &crests(&right(&big(&fine(BLUE, BLUE, at - step / 2.0, 0.0)))),
            26,
        );
        let to = nearest(
            &crests(&right(&big(&fine(BLUE, BLUE, at + step / 2.0, 0.0)))),
            from,
        );
        from as i32 - to as i32
    };
    let (fast, slow) = (pour(0.125), pour(0.375));
    eprintln!("[surge] a band sinks {fast}px on the pull, {slow}px on the release");
    assert!(
        fast >= slow + 2 && slow >= 0,
        "P2 failed: the pour should quicken on the pull, {fast} vs {slow}px"
    );
}

/// A sleeping fine lattice in two poles (see the test's `fine`).
fn lattice_w(a: [f32; 3], b: [f32; 3]) -> ModernPaper {
    ModernPaper {
        spacing: [2.0, 2.0],
        radius: 0.9,
        ..lattice(a, b, 0.0)
    }
}

/// Lift from the bare page along a ray out of the centre of a 128px shot,
/// one sample per step of `(dx, dy)`, `n` samples.
fn ray(buf: &[u8], (dx, dy): (i32, i32), n: i32) -> Vec<i32> {
    (0..n)
        .map(|t| {
            let (x, y) = ((64 + t * dx) as usize, (64 + t * dy) as usize);
            lift(DARK, rgb_w(buf, 128, x, y))
        })
        .collect()
}

/// Where `v` peaks: the highest point within two samples either side, and
/// standing clear (by 10 levels) of the lowest within four.
fn crests(v: &[i32]) -> Vec<usize> {
    (4..v.len().saturating_sub(4))
        .filter(|&i| {
            let near = &v[i - 2..=i + 2];
            let wide = &v[i - 4..=i + 4];
            v[i] == *near.iter().max().unwrap()
                && v[i] - wide.iter().min().unwrap() >= 10
                && (i == 4 || v[i - 1] < v[i])
        })
        .collect()
}

/// Where a ring of samples (3° apart) peaks: the highest point within four
/// samples either way round, standing 40 levels clear of the lowest within
/// eight — a band crossing, not a pixel step.
fn ring_crests(v: &[i32]) -> Vec<usize> {
    let n = v.len();
    let at = |i: usize, d: isize| v[(i as isize + d).rem_euclid(n as isize) as usize];
    (0..n)
        .filter(|&i| {
            (-4..=4).all(|d| at(i, d) < v[i] || (at(i, d) == v[i] && d >= 0))
                && (-8..=8).map(|d| at(i, d)).min().unwrap() + 40 <= v[i]
        })
        .collect()
}

/// The eye of a 256px shot: the mean over 36 rays of each ray's brightest
/// lift between 8 and 22px from the centre, and the mean lift within 3px.
fn eye(buf: &[u8]) -> (i32, i32) {
    let at = |r: f32, a: f32| {
        let (x, y) = (128.0 + r * a.cos(), 128.0 + r * a.sin());
        lift(DARK, rgb_w(buf, 256, x as usize, y as usize))
    };
    let rays = (0..36).map(|k| (10.0 * k as f32).to_radians());
    let ring: i32 = rays
        .clone()
        .map(|a| (8..=22).map(|r| at(r as f32, a)).max().unwrap())
        .sum::<i32>()
        / 36;
    let mid: i32 = rays
        .map(|a| (0..=3).map(|r| at(r as f32, a)).sum::<i32>())
        .sum::<i32>()
        / 144;
    (ring, mid)
}

/// The entry of `at` closest to `to`.
fn nearest(at: &[usize], to: usize) -> usize {
    *at.iter().min_by_key(|&&x| x.abs_diff(to)).unwrap()
}
