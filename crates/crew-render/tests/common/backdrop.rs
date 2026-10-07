//! Fixtures for the backdrop tests (`backdrop_*_headless`): two pages, a few
//! poles, a wash-only and a lattice-only `ModernPaper`, and pixel reads.
//!
//! Compiled into every test target that says `mod common;`, and the targets
//! that do not draw the backdrop use none of it — hence the allow.
#![allow(dead_code)]

use crew_render::{ModernPaper, WashClocks};

/// The near-black aurora page and the light paper page.
pub const DARK: [f32; 4] = [15.0 / 255.0, 17.0 / 255.0, 23.0 / 255.0, 1.0];
pub const LIGHT: [f32; 4] = [244.0 / 255.0, 241.0 / 255.0, 234.0 / 255.0, 1.0];
pub const BLUE: [f32; 3] = [138.0 / 255.0, 180.0 / 255.0, 248.0 / 255.0];
pub const ROSE: [f32; 3] = [249.0 / 255.0, 138.0 / 255.0, 160.0 / 255.0];
pub const VIOLET: [f32; 3] = [197.0 / 255.0, 138.0 / 255.0, 249.0 / 255.0];

/// A wash-only page: dots off so the pools are measured alone.
pub fn wash(phase: f32, wander: f32) -> ModernPaper {
    ModernPaper {
        color_a: BLUE,
        color_b: ROSE,
        dots: 0.0,
        spacing: [16.0, 16.0],
        radius: 2.0,
        wash: 0.6,
        clocks: WashClocks {
            phase,
            wander,
            ..Default::default()
        },
        focus: [0.5, 0.5],
        focus_pull: 0.0,
    }
}

/// A lattice-only page: dots on a 16px pitch, so dot centres sit 0.7px from
/// pixels (8, 8) + 16k (see `paperbg_headless` F1), wash off.
pub fn lattice(a: [f32; 3], b: [f32; 3], phase: f32) -> ModernPaper {
    ModernPaper {
        color_a: a,
        color_b: b,
        dots: 0.3,
        wash: 0.0,
        ..wash(phase, 0.0)
    }
}

/// `m` with its flow fully awake: the current and eddies bending it, the
/// silk lying across it.
pub fn awake(m: ModernPaper) -> ModernPaper {
    with(m, |k| k.live = 1.0)
}

/// `m` with its clocks changed by `f`.
pub fn with(m: ModernPaper, f: impl FnOnce(&mut WashClocks)) -> ModernPaper {
    let mut clocks = m.clocks;
    f(&mut clocks);
    ModernPaper { clocks, ..m }
}

pub fn rgb(buf: &[u8], x: usize, y: usize) -> (i32, i32, i32) {
    rgb_w(buf, 64, x, y)
}

/// [`rgb`] on a shot `w` pixels wide.
pub fn rgb_w(buf: &[u8], w: usize, x: usize, y: usize) -> (i32, i32, i32) {
    let off = (y * w + x) * 4;
    (buf[off] as i32, buf[off + 1] as i32, buf[off + 2] as i32)
}

/// How far apart two pixels are, summed over the channels.
pub fn dist(a: (i32, i32, i32), b: (i32, i32, i32)) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs() + (a.2 - b.2).abs()
}

/// Distance from the bare page, summed over the channels.
pub fn lift(page: [f32; 4], (r, g, b): (i32, i32, i32)) -> i32 {
    let p = |c: f32| (c * 255.0).round() as i32;
    (r - p(page[0])).abs() + (g - p(page[1])).abs() + (b - p(page[2])).abs()
}
