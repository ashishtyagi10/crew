//! The axes and the arithmetic the two spend charts share.
//!
//! `/usage` and `/dash` draw the same two pictures — a week of tokens by hour,
//! and what each day of it cost — from the same buckets, and only `/usage` was
//! saying what their axes were. On the dashboard the heatmap was seven
//! unlabelled bands (is that stripe your morning or your evening?) and the cost
//! curve was a shape with a peak and no dates under it. Both had the row to
//! spare: the division already leaves one under each chart.
//!
//! So the ticks live here, taking the geometry rather than assuming it —
//! the two panes inset their grids differently — and both panes call them.
use crew_render::CellView;

/// Hours the heatmap names: midnight, and every six hours after it.
const TICKS: [usize; 4] = [0, 6, 12, 18];

/// Hour ticks under a heatmap whose grid starts at column `left` and is
/// `grid_w` columns wide, on `row`.
pub(crate) fn hour_ticks(out: &mut Vec<CellView>, left: u16, grid_w: u16, row: u16, cols: u16) {
    let hours = crate::usageledger::HOURS as u16;
    for h in TICKS {
        let col = left + (h as u16 * grid_w) / hours;
        crate::navtext::put_at(
            out,
            &format!("{h:02}"),
            col,
            row,
            cols,
            crew_theme::theme().text_muted,
        );
    }
}

/// The ends of the week under a daily curve: the oldest day on the left, today
/// on the right, clear of `right_pad` columns of air.
pub(crate) fn week_ends(out: &mut Vec<CellView>, row: u16, cols: u16, right_pad: u16) {
    let t = crew_theme::theme();
    let days = crate::usageledger::DAYS.saturating_sub(1);
    let left = format!("{days}d ago");
    crate::navtext::put_at(out, &left, 1, row, cols, t.text_muted);
    let at = cols.saturating_sub(right_pad + 5);
    crate::navtext::put_at(out, "today", at, row, cols, t.text_muted);
}

/// Two shares of a whole as percentages that ADD UP.
///
/// Each was floored on its own, so a 1.92M/0.43M split read `81% / 18%` and a
/// reader who added them got 99 — in a card whose whole job is to account for
/// something. Largest remainder: floor both, and the single point that is left
/// over goes to whichever share was cut by more (the larger share breaks a
/// tie, since it is the one a reader checks against the total).
pub(crate) fn split_pct(a: u64, b: u64) -> (u64, u64) {
    let total = a + b;
    if total == 0 {
        return (0, 0);
    }
    let (fa, fb) = (a * 100 / total, b * 100 / total);
    match 100 - fa - fb {
        0 => (fa, fb),
        _ if a * 100 % total > b * 100 % total => (fa + 1, fb),
        _ if b * 100 % total > a * 100 % total => (fa, fb + 1),
        _ if a >= b => (fa + 1, fb),
        _ => (fa, fb + 1),
    }
}

#[cfg(test)]
#[path = "usageaxis_tests.rs"]
mod tests;
