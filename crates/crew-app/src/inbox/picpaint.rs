//! The open thread's pictures, in the boxes the layout gave them, clipped
//! above the foot: drawn as textures at their own resolution (`pictures`,
//! the reader loop, round 3), each at its own shape in its box, at the left
//! unless the mail centres it. A picture still coming is a quiet box on the
//! paint layer, so the words under it do not seem to start in mid-air.
use crew_render::{CellView, Paint, Picture};

use super::pics::Pic;
use super::{reader, InboxPane, Stage};

impl InboxPane {
    /// Cells, and what is painted under them: the list's motion, the
    /// hairline of a split pane, the boxes of pictures still coming.
    pub fn art(&self, cols: u16, rows: u16, aspect: f32) -> (Vec<CellView>, Vec<Paint>) {
        let cells = self.cells(cols, rows);
        let rows = self.reader_rows(cols, rows);
        let r = match (&self.stage, &self.reading) {
            (Stage::Ready(_), Some(r)) => r,
            (Stage::Ready(_), None) => return (cells, self.list_paint(cols, rows, aspect)),
            _ => return (cells, Vec::new()),
        };
        let mut paint = match self.split(cols) {
            Some(s) => self.list_paint(s.list, rows, aspect),
            None => Vec::new(),
        };
        paint.extend(self.split_paint(cols, rows));
        let (at, width) = self.reader_box(cols);
        let width = reader::text_width(width);
        let (room, quiet) = (f32::from(rows), crew_theme::theme().text_muted);
        for p in &r.laid(width).pics {
            if matches!(self.pics.get(&p.key), Some(Pic::Ready(_))) {
                continue;
            }
            let top = (p.row as f32 - r.scroll as f32).max(0.0);
            let bottom = (p.row as f32 + p.rows as f32 - r.scroll as f32).min(room);
            if bottom > top {
                let at = 2.0 + f32::from(at);
                let bx = Paint::solid(at, top, width as f32, bottom - top, quiet);
                paint.push(Paint { alpha: 0.08, ..bx });
            }
        }
        (cells, paint)
    }

    /// The open thread's pictures that have landed, placed for the renderer.
    pub fn pictures(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Picture> {
        let (Stage::Ready(_), Some(r)) = (&self.stage, &self.reading) else {
            return Vec::new();
        };
        let rows = self.reader_rows(cols, rows);
        let (at, width) = self.reader_box(cols);
        let width = reader::text_width(width);
        let clip = [f32::from(at), 0.0, f32::from(cols), f32::from(rows)];
        let mut out = Vec::new();
        for p in &r.laid(width).pics {
            let Some(px) = self.pics.pixels(&p.key) else {
                continue;
            };
            // Fit in square units (columns across, rows times `aspect`
            // down), then back to rows.
            let (iw, ih, bw, bh) = (px.w as f32, px.h as f32, width as f32, p.rows as f32);
            let scale = (bw / iw).min(bh * aspect / ih);
            let (w, h) = (iw * scale, ih * scale / aspect);
            let x = 2.0 + f32::from(at) + if p.mid { (bw - w) / 2.0 } else { 0.0 };
            let y = p.row as f32 - r.scroll as f32 + (bh - h) / 2.0;
            out.push(Picture {
                pixels: px,
                x,
                y,
                w,
                h,
                clip,
            });
        }
        out
    }
}
