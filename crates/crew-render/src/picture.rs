//! Pictures as pictures: a pane hands over its pictures' pixels and the
//! boxes they go in, and the renderer draws each as one texture, sampled
//! smoothly at the display's own resolution (the reader loop, round 3: "how
//! image render"). Until now a picture was a mosaic of coloured quads, a
//! couple of pixels each, which a photo on a Retina screen showed plainly.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// A picture's pixels: RGBA, row-major, `w * h * 4` bytes, the colour the
/// file holds (sRGB), kept premultiplied: filtered straight, a transparent
/// edge pulled in the black of its empty pixels and wore a dark fringe.
/// Named by an id of its own, so the texture made from it is made once,
/// however many frames draw it.
pub struct Pixels {
    id: u64,
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl Pixels {
    pub fn new(w: u32, h: u32, mut rgba: Vec<u8>) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        for px in rgba.chunks_exact_mut(4).filter(|p| p[3] < 255) {
            let a = u16::from(px[3]);
            for c in &mut px[..3] {
                *c = (u16::from(*c) * a / 255) as u8;
            }
        }
        Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            w,
            h,
            rgba,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

/// A picture placed in a pane, in cells from the pane's origin: the box it
/// fills, and the part of the pane it may show in (`x0, y0, x1, y1`).
#[derive(Clone)]
pub struct Picture {
    pub pixels: Arc<Pixels>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub clip: [f32; 4],
}

/// One picture to draw, in pixels: where on the target (`x, y, w, h`) and
/// which part of the picture (`u0, v0, u1, v1`), the clip already taken.
pub(crate) struct Placed {
    pub pixels: Arc<Pixels>,
    pub rect: [f32; 4],
    pub uv: [f32; 4],
}

/// `p`, in a pane at `(ox, oy)` px with `cw` by `ch` px cells, clipped: none
/// when nothing of it shows.
pub(crate) fn place(p: &Picture, ox: f32, oy: f32, cw: f32, ch: f32) -> Option<Placed> {
    let (x0, y0) = (ox + p.x * cw, oy + p.y * ch);
    let (x1, y1) = (x0 + p.w * cw, y0 + p.h * ch);
    let [cx0, cy0, cx1, cy1] = p.clip;
    let (vx0, vy0) = (x0.max(ox + cx0 * cw), y0.max(oy + cy0 * ch));
    let (vx1, vy1) = (x1.min(ox + cx1 * cw), y1.min(oy + cy1 * ch));
    if vx1 <= vx0 || vy1 <= vy0 || x1 <= x0 || y1 <= y0 {
        return None;
    }
    let u = |x: f32| (x - x0) / (x1 - x0);
    let v = |y: f32| (y - y0) / (y1 - y0);
    Some(Placed {
        pixels: Arc::clone(&p.pixels),
        rect: [vx0, vy0, vx1 - vx0, vy1 - vy0],
        uv: [u(vx0), v(vy0), u(vx1), v(vy1)],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_placed_in_pixels_and_clipped_with_its_texture_coordinates() {
        let pixels = Arc::new(Pixels::new(2, 2, vec![0; 16]));
        assert_ne!(
            pixels.id(),
            Pixels::new(1, 1, vec![0; 4]).id(),
            "each its own"
        );
        let p = Picture {
            pixels,
            x: 1.0,
            y: 2.0,
            w: 4.0,
            h: 4.0,
            clip: [0.0, 0.0, 10.0, 4.0],
        };
        let got = place(&p, 100.0, 50.0, 10.0, 20.0).expect("half shows");
        assert_eq!(got.rect, [110.0, 90.0, 40.0, 40.0]);
        assert_eq!(got.uv, [0.0, 0.0, 1.0, 0.5], "its top half");
        let gone = Picture { y: 5.0, ..p };
        assert!(place(&gone, 100.0, 50.0, 10.0, 20.0).is_none());
    }
}
