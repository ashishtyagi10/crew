//! Minimized pane thumbnails: the bottom strip of fieldset cards for panes
//! demoted out of the full grid (LRU). Each card shows the pane title and a
//! marker — the attention glyph when the pane needs you, else the quiet
//! activity dot — enough to track a pane at a glance and click to restore it.
use crew_render::{CellView, PaneScene};

use crate::attention::Attention;
use crate::layout::Rect;
use crate::pane::Pane;
use crate::panecard::push_card;

/// The one-cell marker for a thumbnail: the attention glyph (bell colour,
/// blinking on the shared clock) supersedes the activity dot; a marker in its
/// blink-off phase draws nothing, like the nav rows.
pub fn strip_marker(
    activity: bool,
    attention: Option<Attention>,
    now: u64,
) -> Option<(char, (u8, u8, u8))> {
    let t = crew_theme::theme();
    match attention {
        Some(a) => a.visible(now).then(|| (a.glyph(), t.bell)),
        None => activity.then_some(('●', t.activity)),
    }
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
        let marker = strip_marker(p.activity, p.attention, now);
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
        assert_eq!(strip_marker(true, Some(a), 0), Some(('✓', t.bell)));
        assert_eq!(strip_marker(true, None, 0), Some(('●', t.activity)));
        assert_eq!(strip_marker(false, None, 0), None);
    }

    #[test]
    fn marker_blinks_off_mid_pulse() {
        let a = Attention {
            kind: NotifyKind::Bell,
            at_ms: 0,
        };
        assert_eq!(strip_marker(false, Some(a), BLINK_MS), None);
        assert!(strip_marker(false, Some(a), 2 * BLINK_MS).is_some());
    }
}
