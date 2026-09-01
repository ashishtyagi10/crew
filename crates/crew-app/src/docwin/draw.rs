//! Drawing one document window: the same card, the same rungs, the same paint
//! layer the viewer pane draws through — a window rather than a tile is the
//! only difference, and it is a rect.
use crew_render::PaneScene;

use super::{DocWindow, MARGIN};
use crate::layout::Rect;
use crate::viewpane::ViewPane;

/// The document, framed, as scenes — everything about the window's picture
/// that does not need a window. Separated so it can be rendered off-screen and
/// looked at: a surface that only exists on a real display is a surface with
/// no picture of itself.
pub(crate) fn scenes(
    rect: Rect,
    cw: f32,
    ch: f32,
    legend: &str,
    view: &ViewPane,
) -> Vec<PaneScene> {
    let mut out: Vec<PaneScene> = Vec::new();
    crate::panelcard::push_card_art(
        &mut out,
        rect,
        cw,
        ch,
        legend,
        crew_theme::theme().legend_off,
        |cols, rows| view.art(cols, rows, ch / cw),
    );
    out
}

impl DocWindow {
    /// Draw a frame. Cheap to call: it is the only thing that draws here, and
    /// nothing schedules it but a key, a resize, or the load landing.
    pub(crate) fn draw(&mut self) {
        let (w, h) = self.renderer.surface_size();
        if w == 0 || h == 0 {
            return;
        }
        let (cw, ch) = self.renderer.cell_size();
        let scale = self.window.scale_factor() as f32;
        let m = MARGIN * scale;
        let rect = Rect {
            x: m,
            y: m,
            w: w as f32 - m * 2.0,
            h: h as f32 - m * 2.0,
        };
        let scenes = scenes(rect, cw, ch, &self.legend(), &self.view);
        // A document window has no busy sweep, no glass sheet and no focus
        // travel: there is one thing in it and it always has the keys.
        self.renderer.set_theme_fade(None);
        self.renderer.set_solid_chrome(Vec::new());
        self.renderer.frame(&scenes);
    }

    /// What the frame's legend says: the file, and where in it you are.
    pub(crate) fn legend(&self) -> String {
        let name = self
            .view
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.view.path.to_string_lossy().into_owned());
        // An editor owes you a standing answer to "is what I typed on disk".
        let name = match self.view.dirty {
            true => format!("{name} \u{25cf}"),
            false => name,
        };
        if let Some(h) = self.hint {
            return format!("{name} \u{00b7} {h}");
        }
        // Typing a URL takes the line the URL was already shown on.
        if let Some(field) = self.link_field_legend() {
            return format!("{name} \u{00b7} {field}");
        }
        // A link's target is invisible in a render; while the cursor is
        // inside one, the frame is where it says so.
        if let Some(url) = self.view.caret_link(self.grid.cols) {
            return format!("{name} \u{00b7} \u{2192} {url}");
        }
        let (back, total) = self.view.position(self.grid.cols, self.grid.rows);
        if total == 0 || total <= usize::from(self.grid.rows) {
            return name;
        }
        // The same reading the pane card's thumb is written from, spelled out
        // here because a window has no card border to draw a thumb on.
        let seen = total.saturating_sub(back);
        let pct = (seen * 100 / total.max(1)).min(100);
        format!("{name} \u{00b7} {pct}%")
    }
}
