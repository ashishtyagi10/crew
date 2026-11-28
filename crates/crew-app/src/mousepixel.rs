//! The pointer in pixels, for programs that set SGR-Pixels (DECSET 1016).
use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// The cursor's 1-based pixel position within terminal pane `i`'s text
    /// area, clamped to it (a drag that leaves the pane pins to its edge).
    /// The window-pixel offset from the content origin (where `cursor_rowcol`
    /// starts counting cells) is scaled to the cell size the pty reports
    /// (`TIOCGWINSZ`, in-band resize), which is rounded to whole pixels, so a
    /// program dividing by that size lands on the cell crew itself resolved.
    pub(crate) fn pane_px(&self, i: usize) -> Option<(u32, u32)> {
        let (cw, ch, ..) = self.frame_geometry()?;
        let rect = self.pane_hit_rects().into_iter().find(|&(n, _)| n == i)?.1;
        let pane = self.panes.get(i)?;
        let PaneContent::Terminal(t) = &pane.content else {
            return None;
        };
        let (x, y) = crate::panefit::content_origin(rect, &pane.content, cw, ch);
        let (pw, ph) = t.pty.cell_px();
        let axis = |off: f32, cell: f32, per: u32, cells: u16| {
            let scaled = off * per as f32 / cell;
            (scaled as u32 + 1).min((u32::from(cells) * per).max(1))
        };
        Some((
            axis(self.cursor.0 - x, cw, pw, pane.grid.cols),
            axis(self.cursor.1 - y, ch, ph, pane.grid.rows),
        ))
    }
}

#[cfg(all(test, unix))]
#[path = "mousepixel_tests.rs"]
mod tests;
