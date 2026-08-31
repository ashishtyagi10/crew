//! Toast notifications: transient cards docked at the top-right of the
//! content area. Where the input-bar status flash is a whisper on the chrome,
//! a toast is the event stepping onto the canvas — it slides in, rests, then
//! dissolves and slides back out. Fed by the same [`crate::notify`] events
//! (and error statuses) that write the LOG, so nothing new to configure.
//!
//! Motion honors the global Motion setting through [`Timeline`]: at `off` the
//! card appears and vanishes with no travel. Every animation window here is
//! bounded, so `wants_animation_frame` goes quiet once the last toast dies —
//! the "an idle crew never repaints" invariant holds.
//!
//! ## A toast you can answer
//!
//! For most of its life this was a card you could only watch. Two things were
//! wrong with that. A message on a 4.8-second timer is unreadable to anyone
//! who reads slowly, is interrupted, or looks up a moment late — WCAG's
//! Pause, Stop, Hide asks that auto-hiding content be pausable, and resting
//! the pointer on the stack now holds every card in it until the pointer
//! leaves. And "agent-7 is waiting" names a pane the user then has to go and
//! find by hand; clicking the card focuses that pane, which is the only thing
//! they wanted to do with the information.
//!
//! Both hang off the rects the frame actually drew ([`Toasts::rects`]),
//! recorded while drawing rather than re-derived, so the hit target can never
//! drift from the card — the discipline `chatfold` uses for card spans.
pub(crate) use crate::toastcard::*;
use crew_render::PaneScene;

use crate::chatwidth::{clip_w, str_w};
use crate::ease::Timeline;
use crate::layout::Rect;

/// How long a toast lives, arrival to removal.
pub(crate) const TTL_MS: u64 = 4_800;
/// Slide-in travel time (scaled by Motion).
pub(crate) const SLIDE_MS: u64 = 260;
/// Tail of the TTL spent dissolving + sliding back out.
pub(crate) const EXIT_MS: u64 = 340;
/// Most cards shown at once; a burst drops the oldest first.
pub(crate) const MAX_SHOWN: usize = 4;

pub(crate) struct Toast {
    pub text: String,
    /// The pane this toast is about, by name — `Some` for every notification
    /// raised by a pane (done, bell, waiting, exited, a watch match).
    ///
    /// Stored as a NAME, not an index: panes open and close while a card is on
    /// screen, and an index captured 4 seconds ago can easily point at a
    /// different pane by the time it is clicked — or past the end of the list.
    pub pane: Option<String>,
    /// Fieldset legend riding the top border ("done", "bell", "waiting"…).
    pub legend: &'static str,
    /// Alert toasts (waiting / exited / errors) border in the bell color.
    pub alert: bool,
    /// How many times this exact card has arrived, `1` for the first. A
    /// repeat does not push a second card ([`Toasts::push_for`]); it counts
    /// here and the card says `\u{d7}N` on its legend.
    pub repeats: usize,
    born_ms: u64,
    slide: Timeline,
}

impl Toast {
    /// How long this card has been on screen, excluding time held under the
    /// pointer. Every expiry decision reads this, never the raw clock.
    fn age(&self, now: u64) -> u64 {
        now.saturating_sub(self.born_ms)
    }
}

/// The live stack, newest last (stacked downward).
#[derive(Default)]
pub(crate) struct Toasts {
    items: Vec<Toast>,
    /// Where each card was drawn last frame, in physical px, newest last —
    /// the same order as `items`. Written by [`push_toasts`] as it draws, so
    /// a hit-test answers about the card the user can actually see.
    rects: Vec<Rect>,
    /// The clock reading at the last `hold` call, while the pointer is on the
    /// stack. `None` when it is not.
    held_since: Option<u64>,
}

impl Toasts {
    pub(crate) fn push(&mut self, text: String, legend: &'static str, alert: bool, now: u64) {
        self.push_for(text, legend, alert, now, None);
    }

    /// [`Self::push`] with the pane the card is about, so clicking it can go
    /// there.
    pub(crate) fn push_for(
        &mut self,
        text: String,
        legend: &'static str,
        alert: bool,
        now: u64,
        pane: Option<String>,
    ) {
        self.prune(now);
        // The same thing said twice is one thing that happened twice. A
        // watched pattern that matches every line, or an agent that finishes
        // three jobs in a second, used to push three identical cards and —
        // at MAX_SHOWN — evict everything else on the stack to do it, so a
        // repeating event could hide every other notification crew had.
        //
        // Matched against ANY live card, not merely the newest: alternating
        // events (`a b a`) repeat just as readily as consecutive ones. The
        // card counts up and starts its life over WHERE IT IS — promoting it
        // to the bottom of the stack would slide every other card, and the
        // pointer may be resting on one of them.
        if let Some(t) = self
            .items
            .iter_mut()
            .find(|t| t.text == text && t.legend == legend && t.pane == pane)
        {
            t.repeats += 1;
            t.born_ms = now;
            return;
        }
        if self.items.len() >= MAX_SHOWN {
            self.items.remove(0);
        }
        self.items.push(Toast {
            text,
            pane,
            legend,
            alert,
            repeats: 1,
            born_ms: now,
            slide: Timeline::start(now, SLIDE_MS, crate::motion::level()),
        });
    }

    /// How many cards are on the stack. The one thing outside this module
    /// that needs to know is whether a notification popped at all — which is
    /// a question only the tests ask, since the renderer walks the stack.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    /// The newest card's `(legend, alert)` — how loudly it is drawn, which is
    /// the part of a notification a caller in another module decides.
    #[cfg(test)]
    pub(crate) fn newest(&self) -> Option<(&'static str, bool)> {
        self.items.last().map(|t| (t.legend, t.alert))
    }

    /// Drop expired toasts.
    pub(crate) fn prune(&mut self, now: u64) {
        self.items.retain(|t| t.age(now) < TTL_MS);
    }

    /// Hold or release the stack for this frame: `point` is the cursor in
    /// physical px, or `None` when the pointer is outside the window.
    ///
    /// A held card's `born_ms` walks forward with the clock, so its age — and
    /// therefore its slide, its dissolve and its expiry — simply stops. The
    /// WHOLE stack holds, not just the card under the pointer: expiring its
    /// neighbours would slide the stack up and move the card out from under
    /// the cursor, which is the one thing a pause must not do.
    ///
    /// Returns whether the pointer is on the stack, so the caller can light
    /// the card and set the pointer shape.
    pub(crate) fn hold(&mut self, now: u64, point: Option<(f32, f32)>) -> bool {
        let on = point.is_some_and(|(x, y)| self.index_at(x, y).is_some());
        if !on {
            self.held_since = None;
            return false;
        }
        // First frame on the stack starts the hold; later frames advance every
        // card by exactly the elapsed time, which freezes their ages.
        if let Some(since) = self.held_since {
            let held = now.saturating_sub(since);
            for t in &mut self.items {
                t.born_ms = t.born_ms.saturating_add(held);
            }
        }
        self.held_since = Some(now);
        true
    }

    /// Which card is at `(x, y)` in physical px, newest first — the stack is
    /// drawn without overlap, so at most one can match either way.
    pub(crate) fn index_at(&self, x: f32, y: f32) -> Option<usize> {
        self.rects
            .iter()
            .rposition(|r| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h)
    }

    /// The pane name the card at `(x, y)` is about, if it is about one.
    pub(crate) fn pane_at(&self, x: f32, y: f32) -> Option<&str> {
        self.items.get(self.index_at(x, y)?)?.pane.as_deref()
    }

    /// Take the card at `(x, y)` off the stack. Returns whether one was there
    /// — so a click that hits no card falls through to whatever is underneath.
    pub(crate) fn dismiss_at(&mut self, x: f32, y: f32) -> bool {
        let Some(i) = self.index_at(x, y) else {
            return false;
        };
        self.items.remove(i);
        self.rects.remove(i);
        true
    }

    /// Whether any card has a frame left to draw *right now*: sliding in, or
    /// inside the exit window (which also covers the removal repaint at Motion
    /// off). A resting toast asks for nothing — poll re-checks each tick, so
    /// frames resume when the exit window opens.
    pub(crate) fn any_live(&self, now: u64) -> bool {
        // A held stack asks for nothing: every age is frozen, so there is no
        // next frame to draw until the pointer moves — and a move repaints.
        if self.held_since.is_some() {
            return false;
        }
        self.items.iter().any(|t| {
            let age = t.age(now);
            age < TTL_MS && (t.slide.live(now) || age >= TTL_MS - EXIT_MS)
        })
    }
}

/// Push one overlay scene per live toast, stacked below the top-right corner
/// of `content`. Overlay scenes get an opaque page-bg backdrop from the
/// overlay pass, so a toast fully occludes whatever pane it rests on.
pub(crate) fn push_toasts(
    scenes: &mut Vec<PaneScene>,
    toasts: &mut Toasts,
    content: Rect,
    cw: f32,
    ch: f32,
    now: u64,
    cursor: Option<(f32, f32)>,
) {
    toasts.prune(now);
    // Hold before laying out, so a held card draws at the age it froze at.
    // `hold` hit-tests LAST frame's rects: the stack has not moved since, and
    // it is the geometry the pointer was actually resting on.
    toasts.hold(now, cursor);
    let hovered = cursor.and_then(|(x, y)| toasts.index_at(x, y));
    toasts.rects.clear();
    let gap = crate::app::gap();
    let max_cols = (((content.w - 2.0 * gap) / cw).floor() as usize).min(MAX_TEXT_COLS + 4);
    let mut y = content.y + gap;
    for (i, t) in toasts.items.iter().enumerate() {
        let text = clip_w(&t.text, max_cols.saturating_sub(4));
        let cols = (str_w(&text) + 4).min(max_cols) as u16;
        if cols < 6 {
            continue;
        }
        let (w, h) = (f32::from(cols) * cw, 3.0 * ch);
        let age = t.age(now);
        // Enter from beyond the right edge, exit the same way. The exit skips
        // at Motion off (the slide Timeline is zero-length there too).
        let travel = w + 2.0 * gap;
        let enter = (1.0 - t.slide.eased(now, crate::ease::out_cubic)) * travel;
        let exit = match crate::motion::level() {
            crate::motion::MotionLevel::Off => 0.0,
            _ => {
                let e = (age.saturating_sub(TTL_MS - EXIT_MS)) as f32 / EXIT_MS as f32;
                e.clamp(0.0, 1.0).powi(2) * travel
            }
        };
        // Text dissolves into the card over the exit window.
        let fade = ((age.saturating_sub(TTL_MS - EXIT_MS)) as f32 / EXIT_MS as f32).clamp(0.0, 1.0);
        let x = content.x + content.w - gap - w + enter + exit;
        // The rect the frame drew, for the next frame's hit-test. Recorded
        // with the slide offsets included: a card halfway in is exactly where
        // it looks, and a click on it lands.
        toasts.rects.push(Rect { x, y, w, h });
        scenes.push(PaneScene {
            cells: card_cells(
                &CardText {
                    text: &text,
                    legend: t.legend,
                    repeats: t.repeats,
                    alert: t.alert,
                    actionable: t.pane.is_some(),
                },
                cols,
                fade,
                hovered == Some(i),
            ),
            x,
            y,
            w,
            h,
            focused: false,
            bordered: false,
            glass: false,
            scan: -1.0,
            overlay: true,
            paint: Vec::new(),
        });
        y += h + gap;
    }
}

impl crate::app::CrewApp {
    /// Answer a left click that landed on a toast. Returns whether one did —
    /// the caller falls through to the pane underneath when it did not.
    ///
    /// A card that names a pane focuses it (restoring it from the nav if it
    /// was minimized, the way every other focus path does); a card that names
    /// none is simply dismissed. Either way the card goes: it has been
    /// answered, and leaving it to time out would say otherwise.
    pub(crate) fn toast_click(&mut self) -> bool {
        let (x, y) = self.cursor;
        let target = self.toasts.pane_at(x, y).map(str::to_string);
        if !self.toasts.dismiss_at(x, y) {
            return false;
        }
        if let Some(name) = target {
            // By name, resolved now — the pane list has had the card's whole
            // life to change under it. Newest match wins: when two panes share
            // a title, the one that raised this is the one that spoke last.
            if let Some(i) = self.panes.iter().rposition(|p| p.title_text() == name) {
                self.focused = i;
                self.input.focused = false;
                // `reconcile_grid` is what restores a minimized pane on focus
                // (see `app.rs`) — going through it keeps the one invariant.
                self.reconcile_grid();
            } else {
                self.set_status(format!("{name} is gone"));
            }
        }
        true
    }
}

#[cfg(test)]
#[path = "toast_tests.rs"]
pub(crate) mod tests;
