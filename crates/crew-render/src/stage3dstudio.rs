//! The studio `mercury`'s chrome mirrors (`stage3dmercury`): a cube
//! environment drawn here, in the theme's colours, for `scene.environment`,
//! which threers prefilters by roughness. A dark floor in the page's colour,
//! a bright horizon in the key's steel, a white softbox overhead, and three tall
//! strip lights: the key's cool steel up-left, the activity's warm copper
//! up-right, the ring's far pole low behind. A mirror shows nothing of its
//! own, so this room is what the liquid metal looks like.
use std::f32::consts::TAU;

use threers::{CubeTexture, TextureFormat};

use crate::stage3dscene::{color, Palette};

/// The cube's edge, in texels: the lobes are mirrors, but curved ones, and
/// threers blurs the cube by roughness besides.
pub(crate) const STUDIO: u32 = 64;

/// sRGB-encode a linear channel into a byte.
fn encode(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

fn lin(c: (u8, u8, u8)) -> [f32; 3] {
    let k = color(c);
    [k.r, k.g, k.b]
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// A soft-edged band: 1 inside `half`, falling to 0 over `soft` beyond it.
fn band(x: f32, half: f32, soft: f32) -> f32 {
    (1.0 - ((x.abs() - half) / soft).clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

/// What the studio shows in direction `d` (unit, y up), linear RGB: a floor
/// in the page's colour, a horizon lifted toward the key, a softbox
/// overhead, and three tall strips, at a fixed azimuth each.
pub(crate) fn studio_at(p: &Palette, d: [f32; 3]) -> [f32; 3] {
    let page = lin(p.page);
    let white = [1.0, 1.0, 1.0];
    let key = mix(lin(p.key), white, 0.7);
    let warm = mix(lin(p.fill), white, 0.25);
    let cool = mix(lin(p.ring.1), white, 0.2);
    let [x, y, z] = d;
    // The room: dark below, a bright band at the horizon, steel above. A
    // mirror's middle shows the horizon, so that band is the metal's body.
    let floor = mix(page, key, 0.15);
    let horizon = mix(page, key, 0.85);
    let sky = mix(page, key, 0.5);
    let mut c = if y < 0.0 {
        mix(horizon, floor, (-y * 2.0).min(1.0))
    } else {
        mix(horizon, sky, (y * 2.5).min(1.0))
    };
    // The softbox: a broad panel straight overhead.
    let top = band(x, 0.35, 0.15) * band(z, 0.5, 0.15) * ((y - 0.7) / 0.1).clamp(0.0, 1.0);
    c = mix(c, white, top);
    // The strips: tall and thin, up-left key, up-right warm rim, low-behind cool.
    let az = x.atan2(z);
    for (at, light, lo, hi) in [
        (-0.9_f32, key, -0.2_f32, 0.8_f32),
        (1.1, warm, -0.3, 0.7),
        (2.7, cool, -0.5, 0.4),
    ] {
        let mut da = (az - at).abs();
        if da > TAU / 2.0 {
            da = TAU - da;
        }
        let tall = band(y - (lo + hi) / 2.0, (hi - lo) / 2.0, 0.08);
        c = mix(c, light, band(da, 0.1, 0.06) * tall);
    }
    c
}

/// The studio as a cube for `scene.environment`: faces +X, −X, +Y, −Y, +Z,
/// −Z, each texel the direction through it, the way a cube map is sampled.
pub(crate) fn studio(p: &Palette) -> CubeTexture {
    let n = STUDIO as usize;
    let face = |f: usize| {
        let mut px = Vec::with_capacity(n * n * 4);
        for row in 0..n {
            for col in 0..n {
                let u = 2.0 * (col as f32 + 0.5) / n as f32 - 1.0;
                let v = 2.0 * (row as f32 + 0.5) / n as f32 - 1.0;
                let d = match f {
                    0 => [1.0, -v, -u],
                    1 => [-1.0, -v, u],
                    2 => [u, 1.0, v],
                    3 => [u, -1.0, -v],
                    4 => [u, -v, 1.0],
                    _ => [-u, -v, -1.0],
                };
                let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                let c = studio_at(p, d.map(|x| x / len));
                px.extend_from_slice(&[encode(c[0]), encode(c[1]), encode(c[2]), 255]);
            }
        }
        px
    };
    CubeTexture::new(
        STUDIO,
        TextureFormat::Rgba8UnormSrgb,
        [face(0), face(1), face(2), face(3), face(4), face(5)],
    )
}

#[cfg(test)]
#[path = "stage3dstudio_tests.rs"]
mod tests;
