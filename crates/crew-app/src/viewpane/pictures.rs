//! Pictures in the viewer: the image a rung decodes, the art it becomes, and
//! the named pictures a markdown document refers to.
//!
//! Split from [`super::render`] for the line cap.
use crate::viewpane::{sticky, ViewPane};
use crew_render::CellView;

impl ViewPane {
    /// The decoded picture this pane is holding, if it is holding one.
    pub(crate) fn image(&self) -> Option<&super::bitmap::Bitmap> {
        match &self.state {
            crate::viewpane::LoadState::Ready { loaded, .. } => loaded.image.as_ref(),
            _ => None,
        }
    }

    /// Cells *and* the paint under them. Every rung but one draws nothing on
    /// the paint layer; the image rung draws almost nothing on the cell one —
    /// a banner naming the file, and the picture itself in the rows below it.
    pub(crate) fn art(
        &self,
        cols: u16,
        rows: u16,
        aspect: f32,
    ) -> (Vec<CellView>, Vec<crew_render::Paint>) {
        let cells = self.cells(cols, rows);
        let Some(bm) = self.image() else {
            // Not a picture FILE, but the document may still name some.
            return (cells, self.named_pictures(cols, rows, aspect));
        };
        let paint = super::bitmap::paint(bm, cols, rows.saturating_sub(1), aspect)
            .into_iter()
            .map(|p| p.shifted(0.0, 1.0))
            .collect();
        (cells, paint)
    }

    /// The pictures this document NAMES, drawn into the rows the layout
    /// reserved for them — clipped to the pane, because a document scrolls and
    /// paint is not clipped by anything else.
    fn named_pictures(&self, cols: u16, rows: u16, aspect: f32) -> Vec<crew_render::Paint> {
        let cache = self.lines_for(cols);
        if cache.pictures.is_empty() {
            return Vec::new();
        }
        let top = self.scroll;
        // Rows a picture must not enter: the sticky heading band owns the
        // first, a live search owns the last. Both are chrome the document
        // scrolls UNDER, and paint is drawn over a cell's background — so
        // without this a picture scrolled halfway off the top is drawn over
        // the band naming the section it is in.
        let y0 = f32::from(u16::from(sticky::label_for(&cache.marks, top).is_some()));
        let y1 = f32::from(rows) - f32::from(u16::from(self.search.is_some()));
        let mut out = Vec::new();
        for p in &cache.pictures {
            // Wholly above or below the window: not merely invisible, but not
            // worth resolving a path or rasterizing for.
            if p.row + p.rows <= top || p.row >= top + usize::from(rows) {
                continue;
            }
            let Some(path) = crate::imgcache::resolve(&p.src, &self.path) else {
                continue;
            };
            let Some(bm) = crate::imgcache::get(&path) else {
                continue;
            };
            let y = p.row as f32 - top as f32;
            out.extend(super::bitmap::paint_at(
                &bm,
                1.0,
                y,
                f32::from(cols).max(2.0) - 2.0,
                p.rows as f32,
                aspect,
                (0.0, y0, f32::from(cols), y1),
            ));
        }
        out
    }
}
