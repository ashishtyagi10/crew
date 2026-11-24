//! The app's half of [`crate::imepreedit`]: which surface a composition is
//! drawn on, where its caret is, and telling the window so.
use crew_render::PaneScene;
use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::app::CrewApp;
use crate::imepreedit::{cell_rect, overlay, Owner};
use crate::pane::PaneContent;

/// Where a composition goes this frame: the scene drawing the owner's cells,
/// the caret cell in them, the column it must stop short of, whether the rest
/// of the row is cleared, and the scene's top-left cell in px.
struct Target {
    scene: usize,
    at: (u16, u16),
    end: u16,
    tail: bool,
    origin: (f32, f32),
}

/// The input bar's caret column, for a bar `cols` wide holding `text`: past
/// the `› ` prompt (column 4, see `inputbar_render`), where the text ends,
/// held at the field's last column once a long line scrolls to follow it.
pub(crate) fn bar_caret_col(text: &str, cols: u16) -> u16 {
    let w = crate::chatwidth::str_w(text) as u16;
    4 + w.min(cols.saturating_sub(6))
}

impl CrewApp {
    /// The surface that takes composed text now, if any (`takes_text`).
    pub(crate) fn ime_owner(&self) -> Option<Owner> {
        match (self.takes_text(), self.input.focused) {
            (false, _) => None,
            (true, true) => Some(Owner::Bar),
            (true, false) => Some(Owner::Pane(self.focused)),
        }
    }

    /// Run once the input bar's scene is the last in `scenes`: draw the
    /// composition over its owner's cells, and while the window takes IME,
    /// tell it where the caret is — only when that rect changed.
    pub(crate) fn draw_preedit(&mut self, scenes: &mut [PaneScene], cw: f32, ch: f32) {
        let Some(owner) = self.ime_owner() else {
            return;
        };
        let Some(t) = self.ime_target(owner, scenes, cw, ch) else {
            return;
        };
        if let Some(pre) = self.ime.preedit.as_ref().filter(|p| p.owner == owner) {
            let ink = (crate::accentink::accent_ink(), crew_theme::theme().page_bg);
            overlay(&mut scenes[t.scene].cells, t.at, t.end, t.tail, pre, ink);
        }
        let r = cell_rect(t.origin, t.at, cw, ch);
        if self.ime.on != Some(true) || self.ime.area == Some(r) {
            return;
        }
        self.ime.area = Some(r);
        if let Some(w) = &self.window {
            let size = PhysicalSize::new(r[2].max(1) as u32, r[3].max(1) as u32);
            w.set_ime_cursor_area(PhysicalPosition::new(r[0], r[1]), size);
        }
    }

    fn ime_target(&self, owner: Owner, scenes: &[PaneScene], cw: f32, ch: f32) -> Option<Target> {
        match owner {
            Owner::Bar => {
                // The bar's own scene, sized as `build_frame` sized it.
                let s = scenes.last()?;
                let cols = (s.w / cw).floor() as u16;
                let rows = (s.h / ch).round() as u16;
                Some(Target {
                    scene: scenes.len() - 1,
                    at: (bar_caret_col(&self.input.text, cols), rows / 2),
                    end: cols.saturating_sub(1),
                    tail: true,
                    origin: (s.x, s.y),
                })
            }
            Owner::Pane(i) => {
                let p = self.panes.get(i)?;
                let PaneContent::Terminal(term) = &p.content else {
                    return None;
                };
                let at = term.pty.ime_cell()?;
                // The content scene `push_pane_scenes` placed at this origin.
                let (x, y) = crate::panefit::origin(p, cw, ch);
                let scene = scenes
                    .iter()
                    .position(|s| s.focused && !s.glass && s.x == x && s.y == y)?;
                Some(Target {
                    scene,
                    at,
                    end: p.grid.cols,
                    tail: false,
                    origin: (x, y - scenes[scene].scroll_px),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "imedraw_tests.rs"]
mod tests;
