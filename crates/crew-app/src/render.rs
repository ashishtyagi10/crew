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

    /// The pane content area and this frame's tile placement — the single
    /// derivation shared by frame building and the mouse hit paths, so they
    /// can never disagree about where a tile sits. `None` until the renderer
    /// reports a real cell size.
    pub(crate) fn placed_grid(&self) -> Option<(Rect, crate::grid::GridRects)> {
        let (_cw, ch, sw, sh, scale) = self.frame_geometry()?;
        let ih = chrome::input_h(ch);
        let content =
            chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        Some((content, compose_grid(content, &self.grid, ch, GAP)))
    }

    /// Returns the actual on-screen rect for every rendered pane, as
    /// `(pane_index, rect)`: the zoomed pane expanded over the whole content
    /// area, or the grid's full tiles + minimized strip thumbnails. This is the
    /// single source of truth for hit-testing and URL rect lookups. Returns empty
    /// when frame geometry is not yet ready.
    pub(crate) fn pane_hit_rects(&self) -> Vec<(usize, Rect)> {
        let Some((content, placed)) = self.placed_grid() else {
            return Vec::new();
        };
        frame_hit_rects(self.zoomed, self.focused, self.panes.len(), content, placed)
    }

    /// Build all PaneScenes for one frame: grid panes in the content area, plus
    /// the docked full-height sidebar when shown, plus the docked bottom input bar.
    pub(crate) fn build_frame(&mut self) -> Vec<PaneScene> {
        let Some((cw, ch, sw, sh, scale)) = self.frame_geometry() else {
            return Vec::new();
        };
        self.reconcile_grid();
        // The pane you're looking at has no unseen activity.
        if !self.input.focused {
            if let Some(p) = self.panes.get_mut(self.focused) {
                p.activity = false;
                p.bell = false;
            }
        }
        // A pane highlights only when the input bar is NOT focused (one active surface).
        let Some((content, placed)) = self.placed_grid() else {
            return Vec::new();
        };
        let mut scenes = if self.zoomed && !self.panes.is_empty() {
            // Zoom: render only the focused pane, expanded to the full content area.
            let i = self.focused.min(self.panes.len() - 1);
            if let Some(r) = zoom_tile(content) {
                relayout(&mut self.panes[i..=i], &[r], cw, ch);
            }
            let f = (!self.input.focused).then_some(0);
            let sel = self.cell_sel.as_ref().filter(|s| s.pane == i);
            build_scenes(
                &self.panes[i..=i],
                f,
                self.broadcast,
                self.last_find.as_deref(),
                sel,
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
                self.cell_sel.as_ref(),
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
                // Advance the animation one frame per *rendered* frame (poll throttles
                // redraws to every ANIM_DIV ticks), so motion stays smooth at 20 fps.
                let tick = self.tick / welcome::ANIM_DIV;
                crate::panecard::push_card(&mut scenes, r, cw, ch, "crew", |cols, rows| {
                    welcome::welcome_cells_animated(cols, rows, tick)
                });
            }
        }

        self.push_sidebar(&mut scenes, sh, scale, cw, ch);

        let ib = chrome::inputbar_rect(content, sh, ch, GAP);
        let ic = (ib.w / cw).floor() as u16;
        let ir = (ib.h / ch).round() as u16;
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
        let matches = crate::suggest::menu_items(&self.input.text);
        if self.input.focused && !matches.is_empty() {
            let mr = crate::cmdmenu::menu_rows(matches.len());
            let mh = mr as f32 * ch;
            let my = (ib.y - mh - GAP).max(0.0);
            scenes.push(PaneScene {
                cells: crate::cmdmenu::menu_card("commands", &matches, self.input.menu_sel, ic, mr),
                x: ib.x,
                y: my,
                w: ib.w,
                h: mh,
                focused: false,
                bordered: false,
                overlay: true,
            });
        }

        // @file mention popup: a "files" fieldset card sitting above the focused
        // crew pane's composer while a mention is being typed. Overlay scene, so
        // the overlay pass backs it with an opaque page background.
        if !self.input.focused {
            if let Some(pane) = self.panes.get(self.focused) {
                if let crate::pane::PaneContent::Chat(c) = &pane.content {
                    if let Some(m) = &c.mention {
                        if !m.matches.is_empty() {
                            let items: Vec<crate::suggest::MenuItem> = m
                                .matches
                                .iter()
                                .map(|p| crate::suggest::MenuItem {
                                    label: format!("@{p}"),
                                    desc: String::new(),
                                    fill: String::new(),
                                    submit: false,
                                })
                                .collect();
                            let r = pane.rect;
                            let cols = (r.w / cw).floor() as u16;
                            let mr = crate::cmdmenu::menu_rows(items.len());
                            let comp = f32::from(crate::chatinput::composer_rows(
                                (r.h / ch).floor() as u16,
                            )) * ch;
                            let mh = f32::from(mr) * ch;
                            let my = (r.y + r.h - comp - mh).max(0.0);
                            scenes.push(PaneScene {
                                cells: crate::cmdmenu::menu_card("files", &items, m.sel, cols, mr),
                                x: r.x,
                                y: my,
                                w: r.w,
                                h: mh,
                                focused: false,
                                bordered: false,
                                overlay: true,
                            });
                        }
                    }
                }
            }
        }

        // Composer palette: a "commands"/"agents" fieldset card sitting above the
        // focused crew pane's composer while a leading `/` or `@` token is being
        // typed (see `chatpalette`). Mutually exclusive with the mention popup
        // above by construction, so both blocks can push independently. Overlay
        // scene, so the overlay pass backs it with an opaque page background.
        if !self.input.focused {
            if let Some(pane) = self.panes.get(self.focused) {
                if let crate::pane::PaneContent::Chat(c) = &pane.content {
                    // `after_edit` clears `palette` whenever it would be empty,
                    // so this is an invariant — guarded to match the mention
                    // block and stay safe if that ever changes.
                    if let Some(p) = c.palette.as_ref().filter(|p| !p.items.is_empty()) {
                        let r = pane.rect;
                        let cols = (r.w / cw).floor() as u16;
                        let mr = crate::cmdmenu::menu_rows(p.items.len());
                        let comp =
                            f32::from(crate::chatinput::composer_rows((r.h / ch).floor() as u16))
                                * ch;
                        let mh = f32::from(mr) * ch;
                        let my = (r.y + r.h - comp - mh).max(0.0);
                        scenes.push(PaneScene {
                            cells: crate::cmdmenu::menu_card(
                                palette_card_title(p.kind),
                                &p.items,
                                p.sel,
                                cols,
                                mr,
                            ),
                            x: r.x,
                            y: my,
                            w: r.w,
                            h: mh,
                            focused: false,
                            bordered: false,
                            overlay: true,
                        });
                    }
                }
            }
        }

        scenes
    }
}

/// The one full-content tile the zoomed view draws — the same gap-inset rect
/// a single grid pane would get. Shared by drawing (`build_frame`) and
/// hit-testing (`frame_hit_rects`) so they can never disagree.
fn zoom_tile(content: Rect) -> Option<Rect> {
    pane_rects_at(1, content.x, content.y, content.w, content.h, GAP)
        .into_iter()
        .next()
}

/// The `(pane_index, rect)` hit list matching what `build_frame` draws.
/// Zoomed, only the focused pane is on screen — expanded over the whole
/// content area — so it is the sole hit target; the grid placement would
/// misroute scrolls and clicks to panes that aren't visible. Otherwise the
/// grid's full tiles plus the minimized strip thumbnails.
pub(crate) fn frame_hit_rects(
    zoomed: bool,
    focused: usize,
    n_panes: usize,
    content: Rect,
    placed: crate::grid::GridRects,
) -> Vec<(usize, Rect)> {
    if zoomed && n_panes > 0 {
        let i = focused.min(n_panes - 1);
        return zoom_tile(content).map(|r| vec![(i, r)]).unwrap_or_default();
    }
    let mut rects = placed.full;
    rects.extend(placed.minimized);
    rects
}

/// Card legend for the composer palette: "commands" for the slash palette,
/// "agents" for the leading-`@` picker.
fn palette_card_title(kind: crate::chatpalette::Kind) -> &'static str {
    match kind {
        crate::chatpalette::Kind::Slash => "commands",
        crate::chatpalette::Kind::Agent => "agents",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chatpalette;
    use crate::grid::GridLayout;

    #[test]
    fn zoomed_hit_rects_are_the_one_drawn_tile_over_the_full_content() {
        // Regression: zoom draws the focused pane over the whole content area,
        // but hit rects used the grid placement — so wheel scrolls over a
        // zoomed pane (e.g. the /md viewer) routed to invisible grid tiles.
        let content = Rect {
            x: 10.0,
            y: 5.0,
            w: 800.0,
            h: 600.0,
        };
        let mut grid = GridLayout::new();
        for i in 0..3 {
            grid.add(i);
        }
        let placed = compose_grid(content, &grid, 16.0, GAP);
        let hits = frame_hit_rects(true, 1, 3, content, placed);
        let drawn = pane_rects_at(1, content.x, content.y, content.w, content.h, GAP)[0];
        assert_eq!(hits, vec![(1, drawn)]);
    }

    #[test]
    fn zoomed_hit_rects_clamp_a_stale_focus_index() {
        let content = Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        };
        let mut grid = GridLayout::new();
        grid.add(0);
        grid.add(1);
        let placed = compose_grid(content, &grid, 16.0, GAP);
        let hits = frame_hit_rects(true, 9, 2, content, placed);
        assert_eq!(hits[0].0, 1, "focus past the end clamps like build_frame");
    }

    #[test]
    fn grid_hit_rects_cover_full_tiles_and_strip_thumbnails() {
        let content = Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        };
        let mut grid = GridLayout::new();
        for i in 0..8 {
            grid.add(i); // 6 full tiles + 2 minimized thumbnails
        }
        let placed = compose_grid(content, &grid, 16.0, GAP);
        let hits = frame_hit_rects(false, 0, 8, content, placed);
        assert_eq!(hits.len(), 8, "every pane keeps a hit rect in grid view");
    }

    fn agents() -> Vec<crew_plugin::AgentInfo> {
        vec![crew_plugin::AgentInfo {
            name: "coder".into(),
            role: "codes".into(),
            model: String::new(),
        }]
    }

    fn legend(cells: &[crew_render::CellView]) -> String {
        let mut row0: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
        row0.sort_by_key(|c| c.col);
        row0.iter().map(|c| c.c).collect()
    }

    #[test]
    fn palette_card_title_matches_kind() {
        assert_eq!(palette_card_title(chatpalette::Kind::Slash), "commands");
        assert_eq!(palette_card_title(chatpalette::Kind::Agent), "agents");
    }

    #[test]
    fn slash_palette_card_shows_commands_legend_and_construct_row() {
        let mut palette = None;
        chatpalette::after_edit(&mut palette, "/mo", &[]);
        let p = palette.unwrap();
        let cells = crate::cmdmenu::menu_card(
            palette_card_title(p.kind),
            &p.items,
            p.sel,
            40,
            crate::cmdmenu::menu_rows(p.items.len()),
        );
        assert!(legend(&cells).contains("commands"));
        assert!(cells.iter().any(|c| c.c == '/'));
    }

    #[test]
    fn agent_palette_card_shows_agents_legend_and_name_row() {
        let mut palette = None;
        chatpalette::after_edit(&mut palette, "@", &agents());
        let p = palette.unwrap();
        let cells = crate::cmdmenu::menu_card(
            palette_card_title(p.kind),
            &p.items,
            p.sel,
            40,
            crate::cmdmenu::menu_rows(p.items.len()),
        );
        assert!(legend(&cells).contains("agents"));
        assert!(cells.iter().any(|c| c.c == 'c')); // "coder" row text
    }
}
