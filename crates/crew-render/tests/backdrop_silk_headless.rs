//! Headless GPU test for the backdrop's SILK: the soft folds of light that
//! lie across an awake page — that they light the lattice, that they lie
//! over the whole page with no centre, that they move calmly, that they have
//! no seam on any clock, that they wear both poles, and that each clock moves
//! them. Each is a pure function of the clocks and the wake the app hands the
//! pass, so each is shot at chosen values and read back. Skips on a GPU-less
//! machine (CI) instead of failing.
mod common;

use common::backdrop::*;
use common::render_offscreen;
use crew_render::{ModernPaper, PaperBgPass, WashClocks};

#[test]
fn backdrop_silk_headless() {
    let Some((device, queue)) = common::gpu() else {
        eprintln!("backdrop_silk_headless: no GPU adapter, skipping");
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
    // A lattice fine enough to read as a field: every pixel is the same
    // distance from its dot, so a fold shows as a smooth ridge, not dots.
    let fine = |a, b, clocks: WashClocks| ModernPaper {
        spacing: [2.0, 2.0],
        radius: 0.9,
        clocks: WashClocks {
            live: 1.0,
            ..clocks
        },
        ..lattice(a, b, 0.0)
    };
    let at = |phase, wander, eddy| WashClocks {
        phase,
        wander,
        eddy,
        ..Default::default()
    };
    let dots16 = |buf: &[u8]| -> Vec<i32> {
        (0..16)
            .map(|i| lift(DARK, rgb(buf, 8 + 16 * (i % 4), 8 + 16 * (i / 4))))
            .collect()
    };

    // F1: the folds light the lattice. One pole, so the turning tint cannot
    // move a pixel and anything that changes is the silk. Asleep the lattice
    // is uniform; awake, its dots rise and fall fold by fold, a crest's
    // carrying far more of the tint than a trough's.
    let span = |v: &[i32]| (*v.iter().min().unwrap(), *v.iter().max().unwrap());
    let (asleep, silk) = (
        dots16(&shot(&lattice(BLUE, BLUE, 0.0))),
        dots16(&shot(&awake(lattice(BLUE, BLUE, 0.25)))),
    );
    let ((a_lo, a_hi), (s_lo, s_hi)) = (span(&asleep), span(&silk));
    eprintln!("[silk] asleep dots {a_lo}..{a_hi}, awake {s_lo}..{s_hi}");
    assert!(
        a_hi - a_lo <= 3,
        "F1 premise: a sleeping lattice is uniform, {asleep:?}"
    );
    assert!(
        s_hi * 10 >= s_lo * 18,
        "F1 failed: the folds should lift their dots, {silk:?}"
    );

    // N1: no centre. The swirl poured into the middle of the page and drew
    // the eye there; the silk lies over all of it. Down the left edge, the
    // middle and the right edge of the page, at several moments, every
    // column crosses folds — and the middle crosses no more than the edges
    // (the swirl's crossed two or three times as many).
    for (i, clocks) in [at(0.1, 0.2, 0.3), at(0.6, 0.7, 0.1), at(0.35, 0.9, 0.8)]
        .into_iter()
        .enumerate()
    {
        let buf = big(&fine(BLUE, BLUE, clocks));
        let folds: Vec<usize> = [12, 64, 116]
            .iter()
            .map(|&x| crests(&column(&buf, x)).len())
            .collect();
        eprintln!("[spread] moment {i}: folds down left/middle/right {folds:?}");
        assert!(
            folds.iter().all(|&n| n >= 2),
            "N1 failed: folds should lie down every column, {folds:?}"
        );
        assert!(
            folds[1] <= folds[0].max(folds[2]) + 1,
            "N1 failed: the middle should be no busier than the edges, {folds:?}"
        );
    }

    // M1: calm. One second of idle drift (the orbit 1/24 of a turn, the slow
    // clock 1/48, the eddies 1/63) moves the page's dots only a little of
    // the way between a trough and a crest: the folds sway, they do not
    // sweep past. The swirl this replaced moved them nearly twice as far in
    // the same second (0.20 of the span, to the silk's 0.11).
    let (t0, t1) = (
        at(0.3, 0.4, 0.5),
        at(0.3 + 1.0 / 24.0, 0.4 + 1.0 / 48.0, 0.5 + 1.0 / 63.0),
    );
    let (f0, f1) = (big(&fine(BLUE, BLUE, t0)), big(&fine(BLUE, BLUE, t1)));
    let field = |buf: &[u8]| -> Vec<i32> {
        (0..32 * 32)
            .map(|i| lift(DARK, rgb_w(buf, 128, 2 + 4 * (i % 32), 2 + 4 * (i / 32))))
            .collect()
    };
    let (v0, v1) = (field(&f0), field(&f1));
    let (lo, hi) = span(&v0);
    let moved =
        v0.iter().zip(&v1).map(|(a, b)| (a - b).abs()).sum::<i32>() as f32 / v0.len() as f32;
    let share = moved / (hi - lo) as f32;
    eprintln!("[calm] a second moves a dot {moved:.1} of a {lo}..{hi} span ({share:.2})");
    assert!(
        hi - lo >= 60,
        "M1 premise: the folds should stand out, {lo}..{hi}"
    );
    assert!(
        share <= 0.15,
        "M1 failed: a second of drift should barely move the folds, {share:.2} of the span"
    );

    // G1: no seam. The step across each clock's wrap (0.999 -> 0) moves each
    // dot about as far as the same-sized step after it (0 -> 0.001); a seam
    // would be a jump of tens of levels.
    let wraps: [(&str, fn(f32) -> WashClocks); 3] = [
        ("orbit", |t| WashClocks {
            phase: t,
            wander: 0.3,
            eddy: 0.6,
            ..Default::default()
        }),
        ("slow", |t| WashClocks {
            phase: 0.3,
            wander: t,
            eddy: 0.6,
            ..Default::default()
        }),
        ("eddy", |t| WashClocks {
            phase: 0.3,
            wander: 0.6,
            eddy: t,
            ..Default::default()
        }),
    ];
    for (name, clock) in wraps {
        let dots = |t: f32| {
            dots16(&shot(&awake(ModernPaper {
                clocks: clock(t),
                ..lattice(BLUE, BLUE, 0.0)
            })))
        };
        let (before, top, after) = (dots(0.999), dots(0.0), dots(0.001));
        for i in 0..16 {
            let (seam, step) = ((top[i] - before[i]).abs(), (after[i] - top[i]).abs());
            assert!(
                seam <= 2 * step + 3,
                "G1 failed: dot {i} jumps {seam} at the {name} clock's wrap, {step} a step later"
            );
        }
    }

    // D1: every clock moves the silk on an awake page — the slow clock sways
    // it and creeps its folds, the third crumples it — and none moves a
    // sleeping one.
    let moved = |a: &[i32], b: &[i32]| a.iter().zip(b).map(|(x, y)| (x - y).abs()).max().unwrap();
    let still = |live, clocks: WashClocks| {
        dots16(&shot(&ModernPaper {
            clocks: WashClocks { live, ..clocks },
            ..lattice(BLUE, BLUE, 0.0)
        }))
    };
    for (name, from, to) in [
        ("slow", at(0.25, 0.0, 0.0), at(0.25, 0.3, 0.0)),
        ("eddy", at(0.25, 0.0, 0.0), at(0.25, 0.0, 0.37)),
    ] {
        let (awoke, asleep) = (
            moved(&still(1.0, from), &still(1.0, to)),
            moved(&still(0.0, from), &still(0.0, to)),
        );
        eprintln!("[clocks] the {name} clock moves a dot {awoke} awake, {asleep} asleep");
        assert_eq!(asleep, 0, "D1 premise: a sleeping page has no silk");
        assert!(
            awoke >= 40,
            "D1 failed: the {name} clock should move the folds, by {awoke}"
        );
    }

    // H1: the folds wear both poles. Down the middle column — square to the
    // lattice tint's axis at phase 0.875, so the tint alone barely changes
    // colour along it — a sleeping page is near one colour, and an awake
    // one's crests run between the two poles.
    let blueness = |buf: &[u8]| -> Vec<(i32, i32)> {
        (0..128)
            .map(|y| {
                let (r, _, b) = rgb_w(buf, 128, 64, y);
                (lift(DARK, (r, 0, b)), b - r)
            })
            .collect()
    };
    let mut blues = Vec::new();
    for clocks in [
        at(0.875, 0.0, 0.0),
        at(0.875, 0.25, 0.7),
        at(0.875, 0.5, 0.5),
        at(0.875, 0.75, 0.2),
    ] {
        let awoke = blueness(&big(&fine(BLUE, ROSE, clocks)));
        let lifts: Vec<i32> = awoke.iter().map(|p| p.0).collect();
        blues.extend(crests(&lifts).into_iter().map(|y| awoke[y].1));
    }
    let asleep = blueness(&big(&ModernPaper {
        spacing: [2.0, 2.0],
        radius: 0.9,
        ..lattice(BLUE, ROSE, 0.875)
    }));
    let (s_lo, s_hi) = span(&asleep.iter().map(|p| p.1).collect::<Vec<_>>());
    let (c_lo, c_hi) = span(&blues);
    eprintln!("[colour] blueness asleep {s_lo}..{s_hi}, awake crests {blues:?}");
    assert!(
        blues.len() >= 4,
        "H1 premise: the column should cross several folds, {blues:?}"
    );
    assert!(
        c_hi - c_lo >= 60,
        "H1 failed: the folds should wear both poles, {blues:?}"
    );
}

/// Lift from the bare page down column `x` of a 128px shot.
fn column(buf: &[u8], x: usize) -> Vec<i32> {
    (0..128)
        .map(|y| lift(DARK, rgb_w(buf, 128, x, y)))
        .collect()
}

/// Where `v` peaks: the highest point within three samples either side, and
/// standing clear (by 15 levels) of the lowest within eight — a fold, not a
/// ripple of the eddies.
fn crests(v: &[i32]) -> Vec<usize> {
    (8..v.len().saturating_sub(8))
        .filter(|&i| {
            let near = &v[i - 3..=i + 3];
            let wide = &v[i - 8..=i + 8];
            v[i] == *near.iter().max().unwrap()
                && v[i] - wide.iter().min().unwrap() >= 15
                && v[i - 1] < v[i]
        })
        .collect()
}
