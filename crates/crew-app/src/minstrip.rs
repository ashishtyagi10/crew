//! Minimized pane thumbnails: the bottom strip of fieldset cards for panes
//! demoted out of the full grid (LRU). Each card shows the pane title and a
//! marker — the attention glyph when the pane needs you, else the quiet
//! activity dot — enough to track a pane at a glance and click to restore it.
use crew_render::{CellView, PaneScene};

use crate::attention::Attention;
use crate::layout::Rect;
use crate::pane::Pane;
use crate::panelcard::push_card;

/// One full pulse of a busy pane's nav dot, in ms.
const PULSE_MS: u64 = 1100;

/// The one-cell marker for a thumbnail. Priority: an attention glyph (bell
/// colour, blinking on the shared clock) supersedes everything; else a busy
/// pane shows a **pulsing** dot (brightness bounces so you can see it working
/// at a glance); else a pane with recent activity shows a **steady** dot; else
/// nothing. A marker in its blink-off phase draws nothing, like the nav rows.
pub fn strip_marker(
    activity: bool,
    attention: Option<Attention>,
    busy: bool,
    now: u64,
) -> Option<(char, (u8, u8, u8))> {
    let t = crew_theme::theme();
    if let Some(a) = attention {
        return a.visible(now).then(|| (a.glyph(), t.bell));
    }
    if busy {
        // Pulse between a dim floor and full activity — always visible, never
        // fully off, so a busy pane reads as alive rather than blinking out.
        let floor = crate::anim::lerp_rgb(t.activity, t.page_bg, 0.6);
        let fg = crate::anim::lerp_rgb(floor, t.activity, crate::anim::tri(now, PULSE_MS));
        return Some(('●', fg));
    }
    activity.then_some(('●', t.activity))
}

/// Push one fieldset card per minimized pane into `scenes`.
pub fn push_min_strip(
    scenes: &mut Vec<PaneScene>,
    panes: &[Pane],
    placed: &[(usize, Rect)],
    cw: f32,
    ch: f32,
) {
    let now = crate::anim::now_ms();
    for &(idx, rect) in placed {
        let Some(p) = panes.get(idx) else { continue };
        let title = p.title_text();
        let marker = strip_marker(p.activity, p.attention, crate::paneview::pane_busy(p), now);
        push_card(scenes, rect, cw, ch, &title, move |cols, _rows| {
            let mut v = Vec::new();
            if let Some((c, fg)) = marker {
                if cols > 0 {
                    v.push(CellView {
                        col: 0,
                        row: 0,
                        c,
                        fg,
                        bg: crew_theme::theme().page_bg,
                        bold: false,
                        italic: false,
                    });
                }
            }
            v
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attention::{Attention, BLINK_MS};
    use crate::notify::NotifyKind;

    #[test]
    fn attention_supersedes_the_activity_dot() {
        let a = Attention {
            kind: NotifyKind::AgentDone,
            at_ms: 0,
        };
        let t = crew_theme::theme();
        assert_eq!(strip_marker(true, Some(a), false, 0), Some(('✓', t.bell)));
        assert_eq!(strip_marker(true, None, false, 0), Some(('●', t.activity)));
        assert_eq!(strip_marker(false, None, false, 0), None);
    }

    #[test]
    fn marker_blinks_off_mid_pulse() {
        let a = Attention {
            kind: NotifyKind::Bell,
            at_ms: 0,
        };
        assert_eq!(strip_marker(false, Some(a), false, BLINK_MS), None);
        assert!(strip_marker(false, Some(a), false, 2 * BLINK_MS).is_some());
    }

    #[test]
    fn busy_pane_pulses_a_dot_that_never_blinks_out() {
        // A busy pane always shows the dot (never None), and its colour changes
        // over the pulse — the trough (dim) differs from the peak (full).
        let trough = strip_marker(false, None, true, 0).expect("busy always shows a dot");
        let peak = strip_marker(false, None, true, PULSE_MS / 2).expect("busy always shows a dot");
        assert_eq!(trough.0, '●');
        assert_ne!(trough.1, peak.1, "the dot pulses between dim and bright");
        // Busy beats a plain activity dot; both are '●' but busy pulses.
        assert!(strip_marker(true, None, true, 0).is_some());
    }
}
