use crew_render::PaneScene;

use crate::app::{CrewApp, GAP};
use crate::chrome;
use crate::layout::{pane_rects_at, Rect};
use crate::pane::{build_scenes, relayout};
use crate::session::pane_at;
use crate::welcome;

impl CrewApp {
    /// `(cell_w, cell_h, surface_w, surface_h, scale)` when the renderer is ready.
    fn frame_geometry(&self) -> Option<(f32, f32, f32, f32, f32)> {
        let r = self.renderer.as_ref()?;
        let (cw, ch) = r.cell_size();
        if cw <= 0.0 || ch <= 0.0 {
            return None;
        }
        let (sw, sh) = r.surface_size();
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        Some((cw, ch, sw as f32, sh as f32, scale))
    }

    /// Sidebar width in physical px (0 when hidden).
    fn nav_px(&self, scale: f32) -> f32 {
        if self.config.show_nav {
            self.config.nav_width * scale
        } else {
            0.0
        }
    }

    /// Grid pane rects packed into the content area (right of the sidebar).
    fn grid_rects(&self) -> Vec<Rect> {
        let Some((_cw, ch, sw, sh, scale)) = self.frame_geometry() else {
            return Vec::new();
        };
        let ih = chrome::input_h(ch);
        let c = chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        pane_rects_at(self.panes.len(), c.x, c.y, c.w, c.h, GAP)
    }

    /// Build all PaneScenes for one frame: grid panes in the content area, plus
    /// the docked full-height sidebar when shown, plus the docked bottom input bar.
    pub(crate) fn build_frame(&mut self) -> Vec<PaneScene> {
        let Some((cw, ch, sw, sh, scale)) = self.frame_geometry() else {
            return Vec::new();
        };
        let ih = chrome::input_h(ch);
        let rects = self.grid_rects();
        relayout(&mut self.panes, &rects, cw, ch);
        let mut scenes = build_scenes(&self.panes, self.focused);

        if self.panes.is_empty() {
            let c = chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
            let wcols = (c.w / cw).floor() as u16;
            let wrows = (c.h / ch).floor() as u16;
            scenes.push(PaneScene {
                cells: welcome::welcome_cells(wcols, wrows),
                x: c.x,
                y: c.y,
                w: c.w,
                h: c.h,
                focused: false,
            });
        }

        if self.config.show_nav {
            let sb = chrome::sidebar_rect(sh, self.nav_px(scale), GAP, ih);
            let sc = (sb.w / cw).floor() as u16;
            let sr = (sb.h / ch).floor() as u16;
            scenes.push(PaneScene {
                cells: self.sidebar.cells(sc, sr),
                x: sb.x,
                y: sb.y,
                w: sb.w,
                h: sb.h,
                focused: false,
            });
        }

        let ib = chrome::inputbar_rect(sw, sh, ih, GAP);
        let ic = (ib.w / cw).floor() as u16;
        let ir = (ib.h / ch).floor() as u16;
        scenes.push(PaneScene {
            cells: self.input.cells(ic, ir),
            x: ib.x,
            y: ib.y,
            w: ib.w,
            h: ib.h,
            focused: false,
        });

        scenes
    }

    /// Which grid pane (if any) sits under the cursor — only inside the content
    /// area, so clicks on the sidebar or input bar do not steal focus.
    pub(crate) fn pane_at_cursor(&self) -> Option<usize> {
        let (_cw, ch, sw, sh, scale) = self.frame_geometry()?;
        let ih = chrome::input_h(ch);
        let c = chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        if !chrome::point_in(c, self.cursor.0, self.cursor.1) {
            return None;
        }
        pane_at(&self.grid_rects(), self.cursor.0, self.cursor.1)
    }
}
