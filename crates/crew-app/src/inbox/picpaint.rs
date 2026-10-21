//! The open thread's pictures on the paint layer, in the boxes the layout
//! gave them, clipped above the foot row: paint is free rectangles, and a
//! picture scrolled half out of the pane must not be drawn over the foot.
use crew_render::{CellView, Paint};

use super::pics::Pic;
use super::{reader, InboxPane, Stage};

impl InboxPane {
    /// Cells, and the pictures drawn under them.
    pub fn art(&self, cols: u16, rows: u16, aspect: f32) -> (Vec<CellView>, Vec<Paint>) {
        let cells = self.cells(cols, rows);
        let r = match (&self.stage, &self.reading) {
            (Stage::Ready(_), Some(r)) => r,
            (Stage::Ready(_), None) => return (cells, self.list_paint(cols, rows)),
            _ => return (cells, Vec::new()),
        };
        let width = reader::text_width(cols);
        let laid = r.laid(width);
        let room = rows.saturating_sub(1) as usize;
        let clip = (0.0, 0.0, f32::from(cols), room as f32);
        let mut paint = Vec::new();
        for p in &laid.pics {
            if p.row + p.rows <= r.scroll || p.row >= r.scroll + room {
                continue;
            }
            let Some(Pic::Ready(bm)) = self.pics.get(&p.key) else {
                continue;
            };
            let y = p.row as f32 - r.scroll as f32;
            let (w, h) = (width as f32, p.rows as f32);
            paint.extend(crate::viewpane::bitmap::paint_at(
                bm, 2.0, y, w, h, aspect, clip,
            ));
        }
        (cells, paint)
    }
}
