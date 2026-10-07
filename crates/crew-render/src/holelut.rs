//! The black hole's light paths, traced once.
//!
//! The backdrop's black hole never moves and neither does the eye looking at
//! it, so how every ray bends round it is fixed: only the disk turns. That
//! lets the bending be the real thing, Schwarzschild's — the lensed far side
//! of the disk arching over the shadow and under it, the thin photon ring —
//! traced here on the CPU when the pass is built, rather than marched per
//! pixel every frame.
//!
//! A ray that passes the hole at impact parameter `b` stays in one plane.
//! In it, `u = 1/r` obeys `u'' + u = 3u²` (units of the hole's mass) as the
//! ray sweeps round by `ψ`, starting at `u = 0`, `u' = 1/b` far away. The
//! table holds `u(b, ψ)`; the shader works out at which `ψ` a pixel's ray
//! crosses the disk's plane, and reads the radius it crosses at from here.
//! A ray that fell in reads [`FELL`], one that got away reads 0 (infinitely
//! far): both are outside any disk, so filtering across them is harmless.

/// Impact parameters covered, `0..=B_MAX` (the disk ends well inside it).
pub(crate) const B_MAX: f32 = 24.0;
/// Sweep covered, `0..=PSI_MAX`: three half-turns, so a ray's first three
/// crossings of the disk's plane — the direct image and the two lensed ones.
pub(crate) const PSI_MAX: f32 = 3.0 * std::f32::consts::PI;
/// Table size: `W` impact parameters across, `H` sweep angles down.
pub(crate) const W: u32 = 384;
pub(crate) const H: u32 = 512;
/// `u` past the horizon (`r < 2`): the ray fell in.
pub(crate) const FELL: f32 = 1.0;

/// RK4 sub-steps between two rows: the ray near the photon sphere turns
/// sharply, and a coarse step would smear the ring.
const SUBSTEPS: u32 = 24;

/// `u(ψ)` for one impact parameter, sampled at the table's `H` rows.
pub(crate) fn column(b: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; H as usize];
    if b <= 0.0 {
        out[1..].fill(FELL);
        return out;
    }
    let dpsi = PSI_MAX as f64 / (H - 1) as f64;
    let h = dpsi / SUBSTEPS as f64;
    let acc = |u: f64| 3.0 * u * u - u;
    let (mut u, mut v) = (0.0f64, 1.0 / b as f64);
    // Fallen or gone, a ray stays that way.
    let mut end: Option<f32> = None;
    for row in out.iter_mut().skip(1) {
        if let Some(e) = end {
            *row = e;
            continue;
        }
        for _ in 0..SUBSTEPS {
            let (k1u, k1v) = (v, acc(u));
            let (k2u, k2v) = (v + 0.5 * h * k1v, acc(u + 0.5 * h * k1u));
            let (k3u, k3v) = (v + 0.5 * h * k2v, acc(u + 0.5 * h * k2u));
            let (k4u, k4v) = (v + h * k3v, acc(u + h * k3u));
            u += h / 6.0 * (k1u + 2.0 * k2u + 2.0 * k3u + k4u);
            v += h / 6.0 * (k1v + 2.0 * k2v + 2.0 * k3v + k4v);
            if u >= 0.5 || u <= 0.0 {
                break;
            }
        }
        *row = match u {
            u if u >= 0.5 => FELL,
            u if u <= 0.0 => 0.0,
            u => u as f32,
        };
        if u >= 0.5 || u <= 0.0 {
            end = Some(*row);
        }
    }
    out
}

/// The whole table as half floats, row by row (`ψ` down, `b` across) —
/// `R16Float` is filterable everywhere, a 32-bit float is not.
pub(crate) fn table() -> Vec<u16> {
    let cols: Vec<Vec<f32>> = (0..W)
        .map(|x| column(B_MAX * x as f32 / (W - 1) as f32))
        .collect();
    (0..H as usize)
        .flat_map(|y| cols.iter().map(move |c| half(c[y])))
        .collect()
}

/// `x` as an IEEE half, round to nearest. Only `0..=1` is ever stored, so
/// the normal range (and zero) is all this needs to handle.
pub(crate) fn half(x: f32) -> u16 {
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    if x == 0.0 || exp <= 0 {
        return 0;
    }
    let mant = bits & 0x7f_ffff;
    let rounded = ((exp as u32) << 10 | mant >> 13) + ((mant >> 12) & 1);
    rounded as u16
}

/// The table's texture, its view and a clamped linear sampler — empty until
/// [`upload`] fills it, which the pass does on its first frame (it is built
/// before it is handed a queue).
pub(crate) fn texture(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hole_lut"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("hole_lut"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    (tex, view, sampler)
}

/// Trace the table and write it into `tex`.
pub(crate) fn upload(queue: &wgpu::Queue, tex: &wgpu::Texture) {
    let data = table();
    // SAFETY: u16 is Pod (no padding, valid for any bit pattern).
    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2) };
    queue.write_texture(
        tex.as_image_copy(),
        bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(W * 2),
            rows_per_image: Some(H),
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
}

#[cfg(test)]
#[path = "holelut_tests.rs"]
mod tests;
