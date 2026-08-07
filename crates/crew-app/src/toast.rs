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
use crew_render::{CellView, PaneScene};

use crate::anim::lerp_rgb;
use crate::boxdraw::titled_card;
use crate::chatwidth::{clip_w, place_row, str_w};
use crate::ease::Timeline;
use crate::layout::Rect;

/// How long a toast lives, arrival to removal.
pub(crate) const TTL_MS: u64 = 4_800;
/// Slide-in travel time (scaled by Motion).
const SLIDE_MS: u64 = 260;
/// Tail of the TTL spent dissolving + sliding back out.
pub(crate) const EXIT_MS: u64 = 340;
/// Most cards shown at once; a burst drops the oldest first.
pub(crate) const MAX_SHOWN: usize = 4;
/// Widest text a card will hold before clipping (display columns).
const MAX_TEXT_COLS: usize = 46;

pub(crate) struct Toast {
    pub text: String,
    /// Fieldset legend riding the top border ("done", "bell", "waiting"…).
    pub legend: &'static str,
    /// Alert toasts (waiting / exited / errors) border in the bell color.
    pub alert: bool,
    born_ms: u64,
    slide: Timeline,
}

/// The live stack, newest last (stacked downward).
#[derive(Default)]
pub(crate) struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub(crate) fn push(&mut self, text: String, legend: &'static str, alert: bool, now: u64) {
        self.prune(now);
        if self.items.len() >= MAX_SHOWN {
            self.items.remove(0);
        }
        self.items.push(Toast {
            text,
            legend,
            alert,
            born_ms: now,
            slide: Timeline::start(now, SLIDE_MS, crate::motion::level()),
        });
    }

    /// Drop expired toasts.
    pub(crate) fn prune(&mut self, now: u64) {
        self.items
            .retain(|t| now.saturating_sub(t.born_ms) < TTL_MS);
    }

    /// Whether any card has a frame left to draw *right now*: sliding in, or
    /// inside the exit window (which also covers the removal repaint at Motion
    /// off). A resting toast asks for nothing — poll re-checks each tick, so
    /// frames resume when the exit window opens.
    pub(crate) fn any_live(&self, now: u64) -> bool {
        self.items.iter().any(|t| {
            let age = now.saturating_sub(t.born_ms);
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
) {
    toasts.prune(now);
    let gap = crate::app::GAP;
    let max_cols = (((content.w - 2.0 * gap) / cw).floor() as usize).min(MAX_TEXT_COLS + 4);
    let mut y = content.y + gap;
    for t in &toasts.items {
        let text = clip_w(&t.text, max_cols.saturating_sub(4));
        let cols = (str_w(&text) + 4).min(max_cols) as u16;
        if cols < 6 {
            continue;
        }
        let (w, h) = (f32::from(cols) * cw, 3.0 * ch);
        let age = now.saturating_sub(t.born_ms);
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
        scenes.push(PaneScene {
            cells: card_cells(&text, t.legend, t.alert, cols, fade),
            x: content.x + content.w - gap - w + enter + exit,
            y,
            w,
            h,
            focused: false,
            bordered: false,
            glass: false,
            scan: -1.0,
            overlay: true,
        });
        y += h + gap;
    }
}

/// The 3-row fieldset card: legend on the top border, one text row inside.
fn card_cells(text: &str, legend: &str, alert: bool, cols: u16, fade: f32) -> Vec<CellView> {
    let t = crew_theme::theme();
    let border = if alert { t.bell } else { t.border_normal };
    let legend_fg = if alert { t.bell } else { t.legend_off };
    let mut cells = titled_card(
        cols,
        3,
        legend,
        lerp_rgb(border, t.page_bg, fade),
        lerp_rgb(legend_fg, t.page_bg, fade),
        t.page_bg,
    );
    let fg = lerp_rgb(t.ink, t.page_bg, fade);
    place_row(2, cols - 1, text.chars().map(|c| (c, fg)), |col, c, fg| {
        cells.push(CellView {
            col,
            row: 1,
            c,
            fg,
            bg: t.page_bg,
            bold: false,
            italic: false,
        });
    });
    cells
}

#[cfg(test)]
#[path = "toast_tests.rs"]
mod tests;
