//! Minimized pane thumbnails: the bottom strip of fieldset cards for panes
//! demoted out of the full grid (LRU). Each card shows the pane title and an
//! activity dot — enough to track a pane at a glance and click to restore it.
use crew_render::{CellView, PaneScene};

use crate::layout::Rect;
use crate::pane::Pane;
use crate::panecard::push_card;

/// Push one fieldset card per minimized pane into `scenes`.
pub fn push_min_strip(
    scenes: &mut Vec<PaneScene>,
    panes: &[Pane],
    placed: &[(usize, Rect)],
    cw: f32,
    ch: f32,
) {
    for &(idx, rect) in placed {
        let Some(p) = panes.get(idx) else { continue };
        let title = p.title_text();
        let activity = p.activity;
        push_card(scenes, rect, cw, ch, &title, move |cols, _rows| {
            let mut v = Vec::new();
            if activity && cols > 0 {
                v.push(CellView {
                    col: 0,
                    row: 0,
                    c: '●',
                    fg: crew_theme::theme().activity,
                    bg: crew_theme::theme().page_bg,
                    bold: false,
                    italic: false,
                });
            }
            v
        });
    }
}
