use crew_render::PaneScene;

use crate::app::{CrewApp, GAP};
use crate::chrome;
use crate::layout::{pane_rects_at, Rect};
use crate::panefit::{relayout, relayout_one};
use crate::paneview::{build_scenes, full_scenes};
use crate::welcome;

impl CrewApp {
    /// Build all PaneScenes for one frame: grid panes in the content area, plus
    /// the docked full-height sidebar when shown, plus the docked bottom input bar.
    pub(crate) fn build_frame(&mut self) -> Vec<PaneScene> {
        let Some((cw, ch, sw, sh, scale)) = self.frame_geometry() else {
            return Vec::new();
        };
        self.reconcile_grid();
        self.mark_focused_seen();
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
                let hint = self.restore_hint;
                crate::panecard::push_card(&mut scenes, r, cw, ch, "crew", |cols, rows| {
                    welcome::welcome_cells_animated(cols, rows, tick, hint)
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
        // input bar when slash input matches a real command (or its value
        // picker); otherwise the same slot shows the live palette preview of
        // what Enter will do instead. `input_preview` stays silent for ALL
        // `/`-led text (submit_input routes every such line to slash dispatch,
        // never to route_bare — even a path like `/bin/echo hi` that the
        // slash palette has no rows for), so this branch is only ever reached
        // for non-slash input. An overlay scene so the overlay pass backs it
        // with black — a box on the canvas, fully opaque.
        let slash_matches = crate::suggest::menu_items(&self.input.text);
        let (matches, title) = if !slash_matches.is_empty() {
            (slash_matches, "commands")
        } else {
            (self.input_preview(), "input")
        };
        if self.input.focused && !matches.is_empty() {
            let mr = crate::cmdmenu::menu_rows(matches.len());
            let mh = mr as f32 * ch;
            let my = (ib.y - mh - GAP).max(0.0);
            scenes.push(PaneScene {
                cells: crate::cmdmenu::menu_card(title, &matches, self.input.menu_sel, ic, mr),
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
                                &c.input,
                                cols,
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
                        let comp = f32::from(crate::chatinput::composer_rows(
                            &c.input,
                            cols,
                            (r.h / ch).floor() as u16,
                        )) * ch;
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
#[path = "render_tests.rs"]
mod tests;
