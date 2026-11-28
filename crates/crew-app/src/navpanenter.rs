//! A new pane's card grows in: its ink rises from the page colour and its
//! spine draws down, one row after the other.
//!
//! Cells cannot move by less than a row, so the entrance is colour, not
//! geometry (the grid's own slide already carries the pane itself). "New"
//! is narrow on purpose: the memory ([`crate::navpanememo`]) saw the card
//! for the first time on a pass *after* it had already seen the section, and
//! the pane is younger than [`ENTER_MS`]. Panes found when the section came
//! into view, or at start-up, are seeded silently; a re-ordered or un-hidden
//! pane is old. At Motion Off the card simply appears.
use crew_render::CellView;

use crate::motion::MotionLevel;
use crate::navpanemotion::Fx;

/// How long a new card takes to arrive, at full motion.
pub(crate) const ENTER_MS: u64 = 300;

/// A pane born `born` is still young enough at `now` to be entering.
pub(crate) fn is_new(born: u64, now: u64) -> bool {
    now.saturating_sub(born) < ENTER_MS
}

/// `fg` risen toward itself from the page: `since` ms into the entrance, the
/// spine's second row `delay` ms late. Subtle starts only 60% down.
pub(crate) fn rise(
    fg: (u8, u8, u8),
    page: (u8, u8, u8),
    since: u64,
    delay: u64,
    level: MotionLevel,
) -> (u8, u8, u8) {
    let dur = level.scale_ms(ENTER_MS);
    if dur == 0 {
        return fg;
    }
    let depth = if level == MotionLevel::Subtle {
        0.6
    } else {
        1.0
    };
    let t = since.saturating_sub(delay) as f32 / (dur - delay.min(dur - 1)) as f32;
    let left = 1.0 - crate::ease::out_cubic(t);
    crate::anim::lerp_rgb(fg, page, depth * left)
}

/// The card's cells `row`.. as they look `now`, mid-entrance if it is one.
pub(crate) fn apply(mut cells: Vec<CellView>, fx: &Fx, row: u16, now: u64) -> Vec<CellView> {
    let level = crate::motion::level();
    let Some(at) = fx.born_at.filter(|_| fx.on) else {
        return cells;
    };
    let since = now.saturating_sub(at);
    let dur = level.scale_ms(ENTER_MS);
    if since >= dur {
        return cells;
    }
    let page = crew_theme::theme().page_bg;
    for c in &mut cells {
        // The spine's lower row starts a quarter late: it draws down.
        let delay = if c.col == 1 && c.row > row {
            dur / 4
        } else {
            0
        };
        c.fg = rise(c.fg, page, since, delay, level);
    }
    cells
}

#[cfg(test)]
#[path = "navpanenter_tests.rs"]
mod tests;
