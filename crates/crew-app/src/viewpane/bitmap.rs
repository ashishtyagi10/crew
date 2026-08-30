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
    let scaled = match sw.max(sh) > MAX_SAMPLES {
        true => img.resize(
            MAX_SAMPLES,
            MAX_SAMPLES,
            image::imageops::FilterType::Triangle,
        ),
        false => img,
    };
    let rgba = scaled.into_rgba8();
    let (w, h) = rgba.dimensions();
    Some(Bitmap {
        w,
        h,
        px: rgba.pixels().map(|p| p.0).collect(),
        src: (sw, sh),
    })
}

/// The picture as quads, fitted inside a `cols × rows` pane and centred.
/// `aspect` is the frame's `cell_h / cell_w`, which is the only thing that
/// keeps a photo from coming out twice as tall as it is.
pub(crate) fn paint(bm: &Bitmap, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
    let (cols, rows) = (f32::from(cols), f32::from(rows));
    if cols < 1.0 || rows < 1.0 || bm.w == 0 || bm.h == 0 || aspect <= 0.0 {
        return Vec::new();
    }
    // Fit in *square* units — columns across, rows × aspect down — then
    // convert the height back into rows to place it.
    let (iw, ih) = (bm.w as f32, bm.h as f32);
    let (box_w, box_h) = (cols, rows * aspect);
    let scale = (box_w / iw).min(box_h / ih);
    let (draw_w, draw_h) = (iw * scale, ih * scale / aspect);
    let (x0, y0) = ((cols - draw_w) / 2.0, (rows - draw_h) / 2.0);
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
                    out.push(
                        Paint::solid(start, y, x - start, qh, color).at(f32::from(c[3]) / 255.0),
                    );
                }
            }
            run = s.map(|c| (x, c));
        }
    }
    out
}

#[cfg(test)]
#[path = "bitmap_tests.rs"]
mod tests;
