//! Pictures. The rung of the viewer's ladder that is not text at all.
//!
//! Every other rung ends in glyphs, because a terminal's unit is a cell and a
//! cell can only say one character. An image has no reading in that alphabet:
//! the half-block trick every `cat`-an-image tool uses buys two samples per
//! cell and spends the whole colour pair doing it, which is a mosaic of the
//! picture rather than the picture.
//!
//! Crew already draws below the cell, though — [`Paint`] is a rectangle in
//! *fractional* cell units, the layer the charts are on — so an image can be
//! laid down as a grid of small quads at whatever resolution the pane can
//! carry, independent of the font. That is the whole implementation: decode
//! once on the worker thread, downscale to a bounded sample grid there (the
//! winit thread must never touch a 40-megapixel photo), and rasterize the
//! samples into quads at draw time, fitted to the pane and centred.
//!
//! Transparent pixels are dropped rather than composited, so a logo lands on
//! the page it is being read on and stays right when the theme changes.
use crew_render::Paint;

/// The largest sample grid kept from a decoded file, per axis. A pane 200
/// columns wide draws ~600 samples across at [`SAMPLES_PER_CELL`], so this is
/// past what any window can show while keeping the decoded copy to about a
/// megabyte.
const MAX_SAMPLES: u32 = 640;

/// Samples per cell along the x axis. Cells are about twice as tall as they
/// are wide, so this is ~6 rows of samples per row of text — fine detail by
/// the standards of a terminal, and quads cost about what a chart's do.
const SAMPLES_PER_CELL: f32 = 3.0;

/// Below this alpha a sample is not drawn at all: the page shows through,
/// which is what a logo with a transparent ground should do on both a light
/// and a dark theme.
const MIN_ALPHA: u8 = 8;

/// A decoded picture, downscaled to a sample grid on the worker thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Bitmap {
    pub w: u32,
    pub h: u32,
    /// RGBA samples, row-major, `w * h` of them.
    pub px: Vec<[u8; 4]>,
    /// The size the file actually was, for the caption — a thumbnail that
    /// says "1024×768" when it is holding 640×480 samples is lying about the
    /// file.
    pub src: (u32, u32),
}

impl Bitmap {
    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = (y.min(self.h - 1) * self.w + x.min(self.w - 1)) as usize;
        self.px[i]
    }
}

/// Decode `bytes` and downscale. Runs on the viewer's worker thread; `None`
/// when the bytes are not an image this build can read.
pub(crate) fn decode(bytes: &[u8]) -> Option<Bitmap> {
    let img = image::load_from_memory(bytes).ok()?;
    let (sw, sh) = (
        image::GenericImageView::dimensions(&img).0,
        image::GenericImageView::dimensions(&img).1,
    );
    if sw == 0 || sh == 0 {
        return None;
    }
    downscale(img, (sw, sh))
}

/// Reduce to at most [`MAX_SAMPLES`] per axis and flatten to RGBA samples,
/// remembering what size the source really was.
fn downscale(img: image::DynamicImage, src: (u32, u32)) -> Option<Bitmap> {
    let scaled = match src.0.max(src.1) > MAX_SAMPLES {
        true => img.resize(
            MAX_SAMPLES,
            MAX_SAMPLES,
            image::imageops::FilterType::Triangle,
        ),
        false => img,
    };
    let rgba = scaled.into_rgba8();
    let (w, h) = rgba.dimensions();
    (w > 0 && h > 0).then(|| Bitmap {
        w,
        h,
        px: rgba.pixels().map(|p| p.0).collect(),
        src,
    })
}

/// The picture as quads, fitted inside a `cols × rows` pane and centred.
/// `aspect` is the frame's `cell_h / cell_w`, which is the only thing that
/// keeps a photo from coming out twice as tall as it is.
pub(crate) fn paint(bm: &Bitmap, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
    let (w, h) = (f32::from(cols), f32::from(rows));
    paint_at(bm, 0.0, 0.0, w, h, aspect, (w, h))
}

/// The picture fitted into an arbitrary box of the pane's grid and clipped to
/// `clip` — what a terminal needs, where the box is wherever the program's
/// cursor was and half of it may have scrolled off the top.
///
/// Nothing else clips: a pane's cells cannot be drawn outside it, but paint is
/// free rectangles, and a quad reaching past the pane would be drawn over the
/// pane beside it.
pub(crate) fn paint_at(
    bm: &Bitmap,
    x0: f32,
    y0: f32,
    box_w: f32,
    box_h: f32,
    aspect: f32,
    clip: (f32, f32),
) -> Vec<Paint> {
    if box_w < 0.5 || box_h < 0.5 || bm.w == 0 || bm.h == 0 || aspect <= 0.0 {
        return Vec::new();
    }
    // Fit in *square* units — columns across, rows × aspect down — then
    // convert the height back into rows to place it.
    let (iw, ih) = (bm.w as f32, bm.h as f32);
    let scale = (box_w / iw).min(box_h * aspect / ih);
    let (draw_w, draw_h) = (iw * scale, ih * scale / aspect);
    let (x0, y0) = (x0 + (box_w - draw_w) / 2.0, y0 + (box_h - draw_h) / 2.0);
    // Sample grid: as many columns of quads as the pane can carry, and enough
    // rows that each quad is square — the picture's own proportions, not the
    // cell's.
    let nx = (draw_w * SAMPLES_PER_CELL).round().max(1.0).min(iw) as u32;
    let ny = ((nx as f32) * ih / iw).round().max(1.0).min(ih) as u32;
    let (qw, qh) = (draw_w / nx as f32, draw_h / ny as f32);
    let mut out = Vec::new();
    for gy in 0..ny {
        let sy = gy * bm.h / ny;
        let y = y0 + gy as f32 * qh;
        // Runs of one colour become one quad: a screenshot or a logo is
        // mostly flat, and the run merge takes those from tens of thousands
        // of quads to hundreds without changing a pixel of the result.
        let mut run: Option<(f32, [u8; 4])> = None;
        for gx in 0..=nx {
            let s = (gx < nx).then(|| bm.at(gx * bm.w / nx, sy));
            let x = x0 + gx as f32 * qw;
            match (run, s) {
                (Some((_, prev)), Some(c)) if prev == c => continue,
                _ => {}
            }
            if let Some((start, c)) = run.take() {
                if c[3] >= MIN_ALPHA {
                    let color = (c[0], c[1], c[2]);
                    let p =
                        Paint::solid(start, y, x - start, qh, color).at(f32::from(c[3]) / 255.0);
                    out.extend(clipped(p, clip));
                }
            }
            run = s.map(|c| (x, c));
        }
    }
    out
}

/// `p` trimmed to the pane box, or nothing when it falls outside it.
fn clipped(p: Paint, clip: (f32, f32)) -> Option<Paint> {
    let (x0, y0) = (p.x.max(0.0), p.y.max(0.0));
    let (x1, y1) = ((p.x + p.w).min(clip.0), (p.y + p.h).min(clip.1));
    (x1 > x0 && y1 > y0).then_some(Paint {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
        ..p
    })
}

/// Wrap raw pixels — the graphics protocol's `f=24`/`f=32` transmissions,
/// which carry no header — and downscale them the same way a decoded file is.
pub(crate) fn from_raw(data: &[u8], w: u32, h: u32, channels: usize) -> Option<Bitmap> {
    let want = (w as usize)
        .checked_mul(h as usize)?
        .checked_mul(channels)?;
    if w == 0 || h == 0 || data.len() < want {
        return None;
    }
    let px: Vec<u8> = data[..want]
        .chunks_exact(channels)
        .flat_map(|c| [c[0], c[1], c[2], *c.get(3).unwrap_or(&255)])
        .collect();
    let img = image::RgbaImage::from_raw(w, h, px)?;
    downscale(image::DynamicImage::ImageRgba8(img), (w, h))
}

#[cfg(test)]
#[path = "bitmap_tests.rs"]
mod tests;
