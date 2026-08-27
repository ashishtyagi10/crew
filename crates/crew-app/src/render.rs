use crew_render::PaneScene;

use crate::app::{gap, CrewApp};
use crate::chrome;
use crate::layout::{pane_rects_at, Rect};
use crate::panefit::{relayout, relayout_one};
use crate::paneview::{build_scenes, full_scenes};
use crate::welcome;

/// Blend two rects — the zoom transition's only geometry.
fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    let m = |x: f32, y: f32| x + (y - x) * t;
    Rect {
        x: m(a.x, b.x),
        y: m(a.y, b.y),
        w: m(a.w, b.w),
        h: m(a.h, b.h),
    }
}

impl CrewApp {
    /// Build all PaneScenes for one frame: grid panes in the content area, plus
    /// the docked full-height sidebar when shown, plus the docked bottom input bar.
    pub(crate) fn build_frame(&mut self) -> Vec<PaneScene> {
        let Some((cw, ch, sw, sh, scale)) = self.frame_geometry() else {
            return Vec::new();
        };
        // Focus bookkeeping (bracket travel + the CRT ignition sweep) lives
        // in `panecardglow::focus_fx`, diffed there once per frame.
        let now = crate::anim::now_ms();
        // Each card's git badge. `poll` starts at most one `git status` at a
        // time and re-asks a directory every few seconds, so this costs a map
        // lookup on almost every frame it is called on.
        let dirs: Vec<std::path::PathBuf> =
            self.panes.iter().filter_map(|p| p.dir.clone()).collect();
        self.git_fleet.poll(&dirs, now / 1000);
        let focus_t = self.focus_fx(now);
        self.theme_fade_tick(now);
        // Grid glide clock: measured between frames, clamped after idle.
        let glide_dt = crate::glide::frame_dt(now, self.glide_prev_ms);
        self.glide_prev_ms = now;
        self.glide_active = false;
        self.reconcile_grid();
        // Before anything is drawn: a key prompt this frame won't show must
        // not survive holding a secret (see `close_hidden_keyentry`).
        self.close_hidden_keyentry();
        self.mark_focused_seen();
        // Which border button the pointer is on, for the cards below.
        self.publish_hover();
        // A pane highlights only when the input bar is NOT focused (one active surface).
        let Some((content, placed)) = self.placed_grid() else {
            return Vec::new();
        };
        let mut scenes = if self.zoomed && !self.panes.is_empty() {
            // Zoom: render only the focused pane, expanded to the full content area.
            let i = self.focused.min(self.panes.len() - 1);
            if let Some(r) = zoom_tile(content) {
                // Travel out of the tile the pane occupied: at t=0 the zoomed
                // pane is exactly its old self, at t=1 it fills the area.
                let t = self.zoom_anim.eased(now, crate::ease::out_cubic);
                let r = match self.zoom_from {
                    Some(from) if t < 1.0 => lerp_rect(from, r, t).snapped(),
                    _ => r,
                };
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
                focus_t,
                cw,
                ch,
                &self.git_fleet,
            )
        } else {
            // Reflow glide: each pane draws a step closer to its placed tile
            // (`pane.rect` is last frame's drawn rect), so grid changes read
            // as the same cards rearranging. Hit-testing stays on the placed
            // targets — clicks land where the panes are heading.
            let snap = matches!(crate::motion::level(), crate::motion::MotionLevel::Off);
            for &(idx, rect) in &placed.full {
                let p = &mut self.panes[idx];
                // Anything that moved the pane without the spring's knowledge
                // (zoom's own lerp, a drag, a resize) left `rect` and the
                // spring disagreeing; adopt the drawn rect before stepping, or
                // the spring yanks the pane back toward where it last thought
                // it was.
                if p.glide.rect() != p.rect {
                    p.glide.reseed(p.rect);
                }
                let (drawn, settled) = p.glide.step(rect, glide_dt, snap);
                self.glide_active |= !settled;
                relayout_one(&mut self.panes[idx], drawn, cw, ch);
            }
            let f = (!self.input.focused).then_some(self.focused);
            full_scenes(
                &self.panes,
                &placed.full,
                f,
                self.broadcast,
                self.last_find.as_deref(),
                self.cell_sel.as_ref(),
                focus_t,
                (self.focused, self.focus_prev),
                cw,
                ch,
                &self.git_fleet,
            )
        };
        if !self.zoomed {
            crate::minstrip::push_min_strip(&mut scenes, &self.panes, &placed, cw, ch);
        }

        if self.panes.is_empty() {
            // Use the SAME rect a single grid pane would occupy (gap-inset) so the
            // welcome area matches a Cmd+T terminal exactly.
            if let Some(r) = pane_rects_at(1, content.x, content.y, content.w, content.h, gap())
                .first()
                .copied()
            {
                // Advance the animation one frame per *rendered* frame (poll throttles
                // redraws to every ANIM_DIV ticks), so motion stays smooth at 20 fps.
                let tick = self.tick / welcome::ANIM_DIV;
                let hint = self.restore_hint;
                crate::panelcard::push_card(&mut scenes, r, cw, ch, "crew", |cols, rows| {
                    welcome::welcome_cells_animated(cols, rows, tick, hint)
                });
            }
        }

        // Cards on their way out, drawn over the settled grid so a collapsing
        // frame sits above whatever reflowed into its place.
        crate::ghost::prune(&mut self.ghosts, now);
        for g in &self.ghosts {
            let r = g.rect_at(now);
            let t = g.collapse_t(now);
            crate::panelcard::push_ghost(&mut scenes, r, cw, ch, &g.title, t);
        }

        self.push_sidebar(&mut scenes, sh, scale, cw, ch);

        let ib = chrome::inputbar_rect(content, sh, ch, gap());
        let ic = (ib.w / cw).floor() as u16;
        let ir = (ib.h / ch).round() as u16;
        scenes.push(PaneScene {
            cells: self.input.cells(
                ic,
                ir,
                self.active_status(),
                self.focused_pane_name().as_deref(),
            ),
            x: ib.x,
            y: ib.y,
            w: ib.w,
            h: ib.h,
            focused: self.input.focused,
            // The input bar draws its own fieldset card border (with the cwd
            // legend), so it opts out of the GPU rounded border — but it is
            // still a card, so it sits on the same frosted sheet.
            bordered: false,
            glass: true,
            scan: -1.0,
            overlay: false,
        });

        // The page's light follows the focused card. Stepped HERE, after the
        // panes have been glided, so it chases the rect a pane is drawn at
        // this frame rather than the tile it is heading for — the card and
        // the light under it move as one thing. It rides the grid's own frame
        // clock for the same reason.
        let focus_rect = if self.input.focused {
            Some(ib)
        } else {
            self.panes
                .get(self.focused)
                .filter(|p| !p.hidden)
                .map(|p| p.rect)
        };
        self.wash_focus
            .step(focus_rect, (sw, sh), glide_dt, crate::motion::level());

        // Toast cards at the top-right of the content area — pushed before the
        // help early-return so an open help screen doesn't swallow them.
        let cursor = self.cursor_in.then_some(self.cursor);
        crate::toast::push_toasts(&mut scenes, &mut self.toasts, content, cw, ch, now, cursor);

        // Keybindings help overlay, centered over everything.
        if self.help_open {
            let (hw, hh) = crate::help::size();
            let hwp = (hw as f32 * cw).min(sw);
            let hhp = (hh as f32 * ch).min(sh);
            let hx = (sw - hwp) / 2.0;
            let hy = (sh - hhp) / 2.0;
            scenes.push(PaneScene {
                cells: crate::help::help_cells(
                    hw.min((sw / cw) as u16),
                    hh.min((sh / ch) as u16),
                    self.help_scroll,
                ),
                x: hx,
                y: hy,
                w: hwp,
                h: hhp,
                focused: false,
                bordered: false,
                glass: false,
                scan: -1.0,
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
            let my = (ib.y - mh - gap()).max(0.0);
            scenes.push(PaneScene {
                cells: crate::cmdmenu::menu_card(title, &matches, self.input.menu_sel, ic, mr),
                x: ib.x,
                y: my,
                w: ib.w,
                h: mh,
                focused: false,
                bordered: false,
                glass: false,
                scan: -1.0,
                overlay: true,
            });
        }

        // Shortcut hints: one row above the input bar while a bare modifier
        // is rested on. Pushed after the command palette so a held Cmd during
        // a slash command sits above it rather than under it — the newer
        // thing on the canvas is the one being asked about.
        if let Some(text) = self.peek_line(now) {
            let ph = 3.0 * ch;
            let py = (ib.y - ph - gap()).max(0.0);
            scenes.push(PaneScene {
                cells: crate::keypeek::peek_card(&text, ic),
                x: ib.x,
                y: py,
                w: ib.w,
                h: ph,
                focused: false,
                bordered: false,
                glass: false,
                scan: -1.0,
                overlay: true,
            });
        }

        // Attach picker popup: an "attach" fieldset card (agents · skills · files) sitting above the focused
        // crew pane's composer while a mention is being typed. Overlay scene, so
        // the overlay pass backs it with an opaque page background.
        if !self.input.focused {
            if let Some(pane) = self.panes.get(self.focused) {
                if let crate::pane::PaneContent::Chat(c) = &pane.content {
                    // Hidden while Ctrl+R search is open — that popup has the
                    // keys (`ChatPane::on_input`), so it must have the screen.
                    if let Some(m) = c.mention.as_ref().filter(|_| c.histsearch.is_none()) {
                        if !m.matches.is_empty() {
                            let items: Vec<crate::suggest::MenuItem> = m
                                .matches
                                .iter()
                                .map(|e| crate::suggest::MenuItem {
                                    label: format!("@{}", e.token()),
                                    desc: e.desc(),
                                    fill: String::new(),
                                    submit: false,
                                    header: false,
                                    dim: false,
                                    needs: None,
                                    color: None,
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
                                cells: crate::cmdmenu::menu_card("attach", &items, m.sel, cols, mr),
                                x: r.x,
                                y: my,
                                w: r.w,
                                h: mh,
                                focused: false,
                                bordered: false,
                                glass: false,
                                scan: -1.0,
                                overlay: true,
                            });
                        }
                    }
                }
            }
        }

        // Provider-key prompt / composer palette: sitting above the focused
        // crew pane's composer, and mutually exclusive with each other — the
        // key prompt takes precedence when both would otherwise apply,
        // matching `ChatPane::on_input`, where an open `keyentry` swallows
        // every key before the palette ever sees one. Overlay scene, so the
        // overlay pass backs it with an opaque page background.
        if !self.input.focused {
            if let Some(pane) = self.panes.get(self.focused) {
                if let crate::pane::PaneContent::Chat(c) = &pane.content {
                    if let Some(entry) = &c.keyentry {
                        let r = pane.rect;
                        let cols = (r.w / cw).floor() as u16;
                        let comp = f32::from(crate::chatinput::composer_rows(
                            &c.input,
                            cols,
                            (r.h / ch).floor() as u16,
                        )) * ch;
                        let mh = f32::from(entry.rows()) * ch;
                        let my = (r.y + r.h - comp - mh).max(0.0);
                        scenes.push(PaneScene {
                            cells: entry.card(cols),
                            x: r.x,
                            y: my,
                            w: r.w,
                            h: mh,
                            focused: false,
                            bordered: false,
                            glass: false,
                            scan: -1.0,
                            overlay: true,
                        });
                    } else if let Some(f) = &c.find {
                        // Cmd+F transcript find: same placement as the Ctrl+R
                        // search; the two are mutually exclusive by
                        // construction (`ChatPane::on_input` closes one when
                        // the other opens), so their order here is free.
                        let r = pane.rect;
                        let cols = (r.w / cw).floor() as u16;
                        let visible = c.visible_messages();
                        let (cells, mr) = crate::chatfind::card(f, &visible, cols);
                        let comp = f32::from(crate::chatinput::composer_rows(
                            &c.input,
                            cols,
                            (r.h / ch).floor() as u16,
                        )) * ch;
                        let mh = f32::from(mr) * ch;
                        let my = (r.y + r.h - comp - mh).max(0.0);
                        scenes.push(PaneScene {
                            cells,
                            x: r.x,
                            y: my,
                            w: r.w,
                            h: mh,
                            focused: false,
                            bordered: false,
                            glass: false,
                            scan: -1.0,
                            overlay: true,
                        });
                    } else if let Some(h) = &c.histsearch {
                        // Ctrl+R history search: same placement as the
                        // palette; before it in the chain, matching the
                        // key-routing priority in `ChatPane::on_input`.
                        let r = pane.rect;
                        let cols = (r.w / cw).floor() as u16;
                        let (cells, mr) = crate::chathistsearch::card(h, cols);
                        let comp = f32::from(crate::chatinput::composer_rows(
                            &c.input,
                            cols,
                            (r.h / ch).floor() as u16,
                        )) * ch;
                        let mh = f32::from(mr) * ch;
                        let my = (r.y + r.h - comp - mh).max(0.0);
                        scenes.push(PaneScene {
                            cells,
                            x: r.x,
                            y: my,
                            w: r.w,
                            h: mh,
                            focused: false,
                            bordered: false,
                            glass: false,
                            scan: -1.0,
                            overlay: true,
                        });
                    } else if let Some(p) = c.palette.as_ref().filter(|p| !p.items.is_empty()) {
                        // `after_edit` clears `palette` whenever it would be
                        // empty, so this is an invariant — guarded to match
                        // the mention block and stay safe if that ever
                        // changes.
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
                            glass: false,
                            scan: -1.0,
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
    pane_rects_at(1, content.x, content.y, content.w, content.h, gap())
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
/// "attach" for the leading-`@` picker (agents, skills, files).
fn palette_card_title(kind: crate::chatpalette::Kind) -> &'static str {
    match kind {
        crate::chatpalette::Kind::Slash => "commands",
        crate::chatpalette::Kind::Agent => "attach",
        crate::chatpalette::Kind::Model => "models",
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
