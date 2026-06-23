use crew_render::PaneScene;

use crate::app::{CrewApp, GAP};
use crate::chrome;
use crate::layout::{pane_rects_at, Rect};
use crate::pane::relayout;
use crate::paneview::build_scenes;
use crate::welcome;

impl CrewApp {
    /// `(cell_w, cell_h, surface_w, surface_h, scale)` when the renderer is ready.
    pub(crate) fn frame_geometry(&self) -> Option<(f32, f32, f32, f32, f32)> {
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
    pub(crate) fn nav_px(&self, scale: f32) -> f32 {
        if self.config.show_nav {
            self.config.nav_width * scale
        } else {
            0.0
        }
    }

    /// Grid pane rects packed into the content area (right of the sidebar).
    pub(crate) fn grid_rects(&self) -> Vec<Rect> {
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
        // The pane you're looking at has no unseen activity.
        if !self.input.focused {
            if let Some(p) = self.panes.get_mut(self.focused) {
                p.activity = false;
                p.bell = false;
            }
        }
        // A pane highlights only when the input bar is NOT focused (one active surface).
        let mut scenes = if self.zoomed && !self.panes.is_empty() {
            // Zoom: render only the focused pane, expanded to the full content area.
            let i = self.focused.min(self.panes.len() - 1);
            let c = chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
            if let Some(r) = pane_rects_at(1, c.x, c.y, c.w, c.h, GAP).into_iter().next() {
                relayout(&mut self.panes[i..=i], &[r], cw, ch);
            }
            let f = (!self.input.focused).then_some(0);
            build_scenes(
                &self.panes[i..=i],
                f,
                self.broadcast,
                self.last_find.as_deref(),
            )
        } else {
            let rects = self.grid_rects();
            relayout(&mut self.panes, &rects, cw, ch);
            let f = (!self.input.focused).then_some(self.focused);
            build_scenes(&self.panes, f, self.broadcast, self.last_find.as_deref())
        };

        if self.panes.is_empty() {
            // Use the SAME rect a single grid pane would occupy (gap-inset) so the
            // welcome area matches a Cmd+T terminal exactly.
            let c = chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
            if let Some(r) = pane_rects_at(1, c.x, c.y, c.w, c.h, GAP).first() {
                let wcols = (r.w / cw).floor() as u16;
                let wrows = (r.h / ch).floor() as u16;
                scenes.push(PaneScene {
                    cells: welcome::welcome_cells_animated(wcols, wrows, self.tick),
                    x: r.x,
                    y: r.y,
                    w: r.w,
                    h: r.h,
                    focused: false,
                    bordered: true,
                });
            }
        }

        if self.config.show_nav {
            let sb = chrome::sidebar_rect(sh, self.nav_px(scale), GAP);
            let sc = (sb.w / cw).floor() as u16;
            let sr = (sb.h / ch).floor() as u16;
            let pane_rows: Vec<crate::panelist::PaneRow> = self
                .panes
                .iter()
                .enumerate()
                .map(|(i, p)| crate::panelist::PaneRow {
                    index: i + 1,
                    title: p.title_text(),
                    focused: i == self.focused,
                    activity: p.activity,
                })
                .collect();
            scenes.push(PaneScene {
                cells: self.sidebar.cells(sc, sr, &pane_rows),
                x: sb.x,
                y: sb.y,
                w: sb.w,
                h: sb.h,
                focused: false,
                bordered: true,
            });
        }

        let content =
            chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        let ib = chrome::inputbar_rect(content, sh, ih, GAP);
        let ic = (ib.w / cw).floor() as u16;
        let ir = (ib.h / ch).floor() as u16;
        scenes.push(PaneScene {
            cells: self.input.cells(ic, ir, self.active_status()),
            x: ib.x,
            y: ib.y,
            w: ib.w,
            h: ib.h,
            focused: self.input.focused,
            // The input bar draws its own fieldset card border (with the cwd
            // legend), so it opts out of the GPU rounded border.
            bordered: false,
        });

        // Keybindings help overlay, centered over everything.
        if self.help_open {
            let (hw, hh) = crate::help::size();
            let hwp = (hw as f32 * cw).min(sw);
            let hhp = (hh as f32 * ch).min(sh);
            let hx = (sw - hwp) / 2.0;
            let hy = (sh - hhp) / 2.0;
            scenes.push(PaneScene {
                cells: crate::help::help_cells(hw.min((sw / cw) as u16), hh.min((sh / ch) as u16)),
                x: hx,
                y: hy,
                w: hwp,
                h: hhp,
                focused: false,
                bordered: false,
            });
            return scenes;
        }

        // Command palette: a popup just above the input bar when slash input matches.
        let matches = crate::suggest::matches(&self.input.text);
        if self.input.focused && !matches.is_empty() {
            let mr = crate::cmdmenu::menu_rows(matches.len());
            let mh = mr as f32 * ch;
            let my = (ib.y - mh - GAP).max(0.0);
            scenes.push(PaneScene {
                cells: crate::cmdmenu::menu_cells(&matches, self.input.menu_sel, ic, mr),
                x: ib.x,
                y: my,
                w: ib.w,
                h: mh,
                focused: false,
                bordered: false,
            });
        }

        scenes
    }
}
