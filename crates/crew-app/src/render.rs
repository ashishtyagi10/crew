use crew_render::PaneScene;

use crate::app::{CrewApp, GAP};
use crate::chrome;
use crate::grid::compose_grid;
use crate::layout::{pane_rects_at, Rect};
use crate::pane::{relayout, relayout_one};
use crate::paneview::{build_scenes, full_scenes};
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

    /// Returns the actual on-screen rect for every rendered pane (full-size grid
    /// tiles + minimized strip thumbnails), as `(pane_index, rect)`. This is the
    /// single source of truth for hit-testing and URL rect lookups. Returns empty
    /// when frame geometry is not yet ready.
    pub(crate) fn pane_hit_rects(&self) -> Vec<(usize, Rect)> {
        let Some((_cw, ch, sw, sh, scale)) = self.frame_geometry() else {
            return Vec::new();
        };
        let ih = chrome::input_h(ch);
        let content =
            chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        let placed = compose_grid(content, &self.grid, ch, GAP);
        let mut rects = placed.full;
        rects.extend(placed.minimized);
        rects
    }

    /// Build all PaneScenes for one frame: grid panes in the content area, plus
    /// the docked full-height sidebar when shown, plus the docked bottom input bar.
    pub(crate) fn build_frame(&mut self) -> Vec<PaneScene> {
        let Some((cw, ch, sw, sh, scale)) = self.frame_geometry() else {
            return Vec::new();
        };
        self.reconcile_grid();
        let ih = chrome::input_h(ch);
        // The pane you're looking at has no unseen activity.
        if !self.input.focused {
            if let Some(p) = self.panes.get_mut(self.focused) {
                p.activity = false;
                p.bell = false;
            }
        }
        // A pane highlights only when the input bar is NOT focused (one active surface).
        let content =
            chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        let placed = compose_grid(content, &self.grid, ch, GAP);
        let mut scenes = if self.zoomed && !self.panes.is_empty() {
            // Zoom: render only the focused pane, expanded to the full content area.
            let i = self.focused.min(self.panes.len() - 1);
            if let Some(r) = pane_rects_at(1, content.x, content.y, content.w, content.h, GAP)
                .into_iter()
                .next()
            {
                relayout(&mut self.panes[i..=i], &[r], cw, ch);
            }
            let f = (!self.input.focused).then_some(0);
            build_scenes(
                &self.panes[i..=i],
                f,
                self.broadcast,
                self.last_find.as_deref(),
                cw,
                ch,
            )
        } else {
            for &(idx, rect) in &placed.full {
                relayout_one(&mut self.panes[idx], rect, cw, ch);
            }
            let f = (!self.input.focused).then_some(self.focused);
            full_scenes(
                &self.panes,
                &placed.full,
                f,
                self.broadcast,
                self.last_find.as_deref(),
                cw,
                ch,
            )
        };
        if !self.zoomed {
            crate::minstrip::push_min_strip(&mut scenes, &self.panes, &placed.minimized, cw, ch);
        }

        if self.panes.is_empty() {
            // Use the SAME rect a single grid pane would occupy (gap-inset) so the
            // welcome area matches a Cmd+T terminal exactly.
            if let Some(r) = pane_rects_at(1, content.x, content.y, content.w, content.h, GAP)
                .first()
                .copied()
            {
                let tick = self.tick;
                crate::panecard::push_card(&mut scenes, r, cw, ch, "crew", |cols, rows| {
                    welcome::welcome_cells_animated(cols, rows, tick)
                });
            }
        }

        self.push_sidebar(&mut scenes, sh, scale, cw, ch);

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
            overlay: false,
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
                overlay: true,
            });
            return scenes;
        }

        // Command menu: a solid-black "commands" fieldset card just above the
        // input bar when slash input matches. An overlay scene so the overlay
        // pass backs it with black — a box on the canvas, fully opaque.
        let matches = crate::suggest::matches(&self.input.text);
        if self.input.focused && !matches.is_empty() {
            let mr = crate::cmdmenu::menu_rows(matches.len());
            let mh = mr as f32 * ch;
            let my = (ib.y - mh - GAP).max(0.0);
            scenes.push(PaneScene {
                cells: crate::cmdmenu::menu_card(&matches, self.input.menu_sel, ic, mr),
                x: ib.x,
                y: my,
                w: ib.w,
                h: mh,
                focused: false,
                bordered: false,
                overlay: true,
            });
        }

        scenes
    }
}
