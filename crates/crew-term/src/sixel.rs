//! Sixel pictures: what the body of a `DCS … q … ST` says about its own size.
//!
//! A sixel stream is `P1;P2;P3 q <data>`: three numbers (P2 = 1 asks for
//! unset pixels to stay transparent), then data in which each character
//! `?`..`~` is six vertical pixels, `!n` repeats the next one `n` times, `$`
//! returns to the band's left edge, `-` starts the next band six pixels down,
//! `#…` picks or defines a colour, and `"Pan;Pad;Ph;Pv` (raster attributes)
//! may announce the image's pixel size up front.
//!
//! The size is needed BEFORE the picture is decoded — how many rows it
//! occupies decides how far the terminal line-feeds — and that happens on the
//! winit thread, so [`size`] allocates nothing and is bounded: a `!999999999`
//! is clamped, never looped. Decoding itself is [`decode`], on a worker.
#[path = "sixelcolor.rs"]
mod color;
#[path = "sixeldec.rs"]
mod dec;

pub use dec::decode;

/// Largest width or height taken seriously. A sender can claim any number in
/// the raster attributes; nothing here believes more than this.
pub const MAX_DIM: u32 = 10_000;
/// Largest picture in pixels (64 MB of RGBA, a 5K screen's worth):
/// `10000 × 10000` would be 400.
pub const MAX_PIXELS: u64 = 16_000_000;

/// The data after `P1;P2;P3 q`, and P2. `None` when the body is not a sixel
/// header at all (anything but digits and `;` before the `q`).
pub(crate) fn parts(body: &[u8]) -> Option<(u8, &[u8])> {
    let q = body
        .iter()
        .position(|&b| !(b.is_ascii_digit() || b == b';'))?;
    if body[q] != b'q' {
        return None;
    }
    let p2 = body[..q].split(|&b| b == b';').nth(1);
    let p2 = p2.and_then(|s| std::str::from_utf8(s).ok()?.parse().ok());
    Some((p2.unwrap_or(0), &body[q + 1..]))
}

/// Read a run of digits at `*i`, saturating well below overflow.
pub(crate) fn num(d: &[u8], i: &mut usize) -> u32 {
    let mut n = 0u32;
    while let Some(c) = d.get(*i).filter(|c| c.is_ascii_digit()) {
        n = n
            .saturating_mul(10)
            .saturating_add(u32::from(c - b'0'))
            .min(1 << 24);
        *i += 1;
    }
    n
}

/// Read `;`-separated numbers starting at `*i` into a fixed array: the first
/// five are kept (the most any sixel command takes), the rest skipped, and a
/// missing one is 0. No allocation: this runs where rows are decided.
pub(crate) fn nums(d: &[u8], i: &mut usize) -> ([u32; 5], usize) {
    let mut v = [0u32; 5];
    let mut k = 0;
    loop {
        let n = num(d, i);
        if k < v.len() {
            v[k] = n;
            k += 1;
        }
        if d.get(*i) != Some(&b';') {
            return (v, k);
        }
        *i += 1;
    }
}

/// What one pass over the data finds: the raster attributes' size if they
/// carry one, and the extent the data actually draws.
struct Scan {
    raster: Option<(u32, u32)>,
    w: u32,
    h: u32,
}

/// One bounded pass, no allocation (the raster's numbers are read into a
/// fixed array).
fn scan(d: &[u8]) -> Scan {
    let (mut i, mut x, mut w) = (0, 0u32, 0u32);
    let (mut band, mut last, mut raster) = (0u32, None, None);
    while i < d.len() {
        let c = d[i];
        i += 1;
        let mut put = |n: u32, band: u32, x: &mut u32| {
            *x = x.saturating_add(n).min(MAX_DIM);
            w = w.max(*x);
            last = Some(band);
        };
        match c {
            b'"' => {
                let (v, _) = nums(d, &mut i);
                if v[2] > 0 && v[3] > 0 {
                    raster.get_or_insert((v[2].min(MAX_DIM), v[3].min(MAX_DIM)));
                }
            }
            b'#' => {
                nums(d, &mut i);
            }
            b'!' => {
                let n = num(d, &mut i).clamp(1, MAX_DIM);
                if d.get(i).is_some_and(|c| (b'?'..=b'~').contains(c)) {
                    i += 1;
                    put(n, band, &mut x);
                }
            }
            b'$' => x = 0,
            b'-' => (band, x) = (band.saturating_add(1), 0),
            b'?'..=b'~' => put(1, band, &mut x),
            _ => {}
        }
    }
    let h = last.map_or(0, |b| (b + 1).saturating_mul(6));
    Scan { raster, w, h }
}

/// The pixel size of the picture in `body` (params, `q`, data), without
/// decoding it: the raster attributes when they carry a size, else the
/// extent of the data itself. A raster is believed only as far as the data
/// reaches: `"1;1;5000;5000` over one sixel would otherwise cost the worker
/// 100 MB for twenty bytes of input.
pub fn size(body: &[u8]) -> Option<(u32, u32)> {
    let s = scan(parts(body)?.1);
    let (w, h) = s
        .raster
        .map_or((s.w, s.h), |(rw, rh)| (rw.min(s.w), rh.min(s.h)));
    let h = h.min(MAX_DIM);
    (w > 0 && h > 0 && u64::from(w) * u64::from(h) <= MAX_PIXELS).then_some((w, h))
}

/// How tall the data really draws, whatever the raster attributes claim —
/// what the cursor follows (see `place_image`). Zero for an empty picture.
pub fn drawn_height(body: &[u8]) -> u32 {
    parts(body).map_or(0, |(_, d)| scan(d).h.min(MAX_DIM))
}

#[cfg(test)]
#[path = "sixel_tests.rs"]
mod sixel_tests;
