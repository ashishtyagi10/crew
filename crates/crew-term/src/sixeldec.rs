//! Turning a sixel stream into pixels. Pure, bounded, never panics: this runs
//! on a worker on whatever a program wrote, so garbage is ignored, not obeyed.
use super::color;
use super::{num, nums, parts, size, MAX_DIM};

/// Decode `body` (params, `q`, data) to `(width, height, RGBA)`.
///
/// The canvas is [`size`]'s — the same number the terminal already reserved
/// rows from — and anything drawn outside it is clipped, so a lying raster
/// header cannot make the picture bigger than the room made for it.
///
/// Pixels no band touched stay fully transparent whatever P2 says. For
/// P2 = 0/2 the DEC model fills them with the background colour (foot's
/// `sixel.c` says an opaque image "is supposed to erase, with the current
/// background color"; xterm was not consulted). On the pane's own background
/// that looks the same; it differs only over a coloured cell background,
/// where letting the cell show through is the kinder error.
/// Pixel aspect (`Pan;Pad`, and P1 in the header) is ignored: pixels are
/// square, which is what every current encoder assumes.
pub fn decode(body: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let (w, h) = size(body)?;
    let (_, d) = parts(body)?;
    let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
    let mut pal = color::palette();
    let (mut cur, mut x, mut y, mut i) = (0usize, 0u32, 0u32, 0usize);
    while i < d.len() {
        let c = d[i];
        i += 1;
        match c {
            b'"' => {
                nums(d, &mut i);
            }
            b'#' => {
                // `#Pc` selects; `#Pc;Pu;Px;Py;Pz` also defines, but only with
                // all five numbers (as foot), and a Pu that is not 1 or 2
                // defines nothing.
                let (v, n) = nums(d, &mut i);
                cur = (v[0] as usize).min(255);
                if n == 5 {
                    if let Some(rgb) = color::define(v[1], v[2], v[3], v[4]) {
                        pal[cur] = rgb;
                    }
                }
            }
            b'!' => {
                let n = num(d, &mut i).min(MAX_DIM);
                if let Some(&s) = d.get(i).filter(|s| (b'?'..=b'~').contains(s)) {
                    i += 1;
                    paint(&mut out, (w, h), (x, y), s - b'?', n, pal[cur]);
                    x = x.saturating_add(n).min(MAX_DIM);
                }
            }
            b'$' => x = 0,
            b'-' => (x, y) = (0, y.saturating_add(6)),
            b'?'..=b'~' => {
                paint(&mut out, (w, h), (x, y), c - b'?', 1, pal[cur]);
                x = x.saturating_add(1).min(MAX_DIM);
            }
            _ => {}
        }
    }
    Some((w, h, out))
}

/// Set the `bits` column (bit 0 on top) `n` pixels wide at `(x, y)`.
fn paint(out: &mut [u8], (w, h): (u32, u32), (x, y): (u32, u32), bits: u8, n: u32, rgb: [u8; 3]) {
    for dx in 0..n.min(w.saturating_sub(x)) {
        for k in (0..6).filter(|k| bits >> k & 1 == 1) {
            let py = y.saturating_add(k);
            if py < h {
                let o = ((py as usize) * (w as usize) + (x + dx) as usize) * 4;
                out[o..o + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
    }
}
