//! A panel card whose rows sit level with the panes beside it. Split from
//! `panelcard` for the line cap.
use crew_render::{CellView, Paint, PaneScene};

use crate::layout::Rect;

/// [`crate::panelcard::push_card_art`] with its interior `dy` px lower ([`level_dy`]), so its
/// rows sit level with the rows of the panes beside it. The rail's numbers
/// stood 6 px above the pane rows they name (glass survey G3).
pub fn push_card_art_level(
    scenes: &mut Vec<PaneScene>,
    rect: Rect,
    (cw, ch): (f32, f32),
    (legend, title_fg): (&str, (u8, u8, u8)),
    dy: f32,
    content: impl FnOnce(u16, u16) -> (Vec<CellView>, Vec<Paint>),
) {
    crate::panelcard::push_card_art(scenes, rect, cw, ch, legend, title_fg, content);
    if let Some(inner) = scenes.len().checked_sub(2).and_then(|i| scenes.get_mut(i)) {
        inner.y += dy;
        inner.h = (inner.h - dy).max(0.0);
    }
}

/// How far a card's interior drops to put its rows on the rows of the
/// topmost of `panes` — that pane's content origin (`panefit::origin`),
/// measured from the card's own first row. The nav card runs past the
/// input bar to the window's foot, so its OWN remainder is not the panes'.
/// A hit test on the card's rows reads it too, so a click lands on the row
/// drawn.
pub fn level_dy(card: Rect, panes: &[crate::pane::Pane], cw: f32, ch: f32) -> f32 {
    let top = panes
        .iter()
        .filter(|p| !p.hidden && p.rect.h > 0.0)
        .map(|p| p.rect);
    level_with(card, top.min_by(|a, b| a.y.total_cmp(&b.y)), cw, ch)
}

/// [`level_dy`] beside a pane in `beside` (none: no drop).
pub fn level_with(card: Rect, beside: Option<Rect>, cw: f32, ch: f32) -> f32 {
    let drop = |r: Rect| r.y + crate::panefit::slack(r, cw, ch).1 - card.y;
    beside.map_or(0.0, |r| drop(r).clamp(0.0, ch - 1.0))
}

#[cfg(test)]
#[path = "panellevel_tests.rs"]
mod tests;
