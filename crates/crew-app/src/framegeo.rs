//! Frame geometry + hit-testing helpers for the renderer (split from
//! `render.rs` for the 200-line cap): content-area math, nav width, placed
//! grid rects, pane hit rects, focused-seen marking.
use crate::app::{gap, CrewApp};
use crate::chrome;
use crate::grid::compose_grid;
use crate::layout::Rect;
use crate::render::frame_hit_rects;

impl CrewApp {
    /// `(cell_w, cell_h, surface_w, surface_h, scale)` when the renderer is ready.
    pub(crate) fn frame_geometry(&self) -> Option<(f32, f32, f32, f32, f32)> {
        let r = self.renderer.as_ref()?;
        let (cw, ch) = r.cell_size();
        if cw <= 0.0 || ch <= 0.0 {
            return None;
        }
        // Every drawn widget rasterizes at one canvas pixel per device pixel,
        // and this is the one place per frame that knows how big one is.
        crate::plot::device::set_cell_w(cw);
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
        let (cw, ch, sw, sh, scale) = self.frame_geometry()?;
        let ih = chrome::bottom_chrome_h(sh, ch, gap());
        let content =
            chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), gap(), ih);
        Some((content, compose_grid(content, &self.grid, cw, ch, gap())))
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

    /// Take down any provider-key prompt this frame will NOT draw. `render.rs`
    /// draws the masked card for the focused chat pane only, and only while
    /// neither the input bar nor the help overlay is covering it — so any
    /// focus move (a plain click on the input bar, `focus_at_cursor`; a click
    /// on another pane; Cmd+P; the help overlay) would otherwise leave the
    /// prompt open but INVISIBLE while still holding a half-typed secret.
    ///
    /// With the input bar focused that is not merely cosmetic: keys never
    /// reach `ChatPane::on_input` at all, so ordinary typing lands in
    /// `self.input.text` — drawn in plaintext in the bar and in the input
    /// preview card — and Enter pushes the line into the command history,
    /// which is written to disk and replayed as ghost text next launch.
    ///
    /// Acting here rather than REFUSING the focus change is deliberate. The
    /// prompt is hidden by a click anywhere outside its pane, by pane
    /// switching and by the help overlay, not just by the input bar; refusing
    /// all of those would make it a modal the user cannot click out of, and
    /// there is no focus chokepoint to enforce it in (nine call sites set
    /// `input.focused`). Doing it once per frame instead means the buffer
    /// invariant holds by construction, however focus moved.
    ///
    /// HIDDEN IS NOT DISMISSED, and an in-flight browser sign-in is where the
    /// difference bites. The most ordinary step of that flow is: the browser
    /// says "you can close this tab", the user clicks back into crew, and the
    /// activating click lands on the input bar (`hit.rs`). If that cancelled
    /// the flow, a sign-in the user just completed would be thrown away with
    /// nothing on screen to say so — and OpenRouter would be left holding a
    /// real key, minted against the user's account, that crew never stored and
    /// they now have to revoke by hand. Switching panes and the help overlay
    /// have the same shape: none of them is the user abandoning the sign-in.
    ///
    /// So a pane with a live `oauth` keeps BOTH the flow and the prompt, and
    /// the prompt comes back the moment the pane is drawn again. What it does
    /// not keep is the typed buffer ([`crate::keyentry::KeyEntry::forget_typing`]):
    /// no secret outlives the card that showed it, which is the invariant this
    /// function exists for.
    ///
    /// The original Critical still holds, because it was never really about
    /// hiding: a key must not be stored into a prompt the user DISMISSED, and
    /// dismissal has exactly three call sites, all of which drop the receiver
    /// (and say so) rather than relying on this pass — Escape and Submit in
    /// `ChatPane::on_input` via `cancel_oauth`, and closing the pane, which
    /// takes the receiver down with the whole pane. A dropped receiver is what
    /// makes the worker's `send` fail, so the flow ends there and no outcome
    /// can land.
    pub(crate) fn close_hidden_keyentry(&mut self) {
        let drawn = (!self.input.focused && !self.help_open).then_some(self.focused);
        for (i, pane) in self.panes.iter_mut().enumerate() {
            if Some(i) == drawn {
                continue;
            }
            let crate::pane::PaneContent::Chat(c) = &mut pane.content else {
                continue;
            };
            if c.oauth.is_some() {
                // Hidden mid-sign-in: keep the flow, keep the prompt, forget
                // the typing.
                if let Some(e) = c.keyentry.as_mut() {
                    e.forget_typing();
                }
            } else {
                // No flow to protect: dropping the `KeyEntry` drops its
                // buffer with it, the same discard Escape performs.
                c.keyentry = None;
            }
        }
    }

    /// The pane you're looking at has no unseen activity: clear its activity
    /// dot, bell, and attention marker. Skipped while the input bar is focused
    /// — typing in the bar isn't looking at the pane.
    pub(crate) fn mark_focused_seen(&mut self) {
        if self.input.focused {
            return;
        }
        if let Some(p) = self.panes.get_mut(self.focused) {
            p.activity = false;
            p.bell = false;
            p.attention = None;
        }
    }
}
