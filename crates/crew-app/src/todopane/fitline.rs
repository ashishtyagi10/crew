//! How WIDE: where a todo's title wraps, whether its chips fit on the same
//! line as the title or stack under it, and where the right-hand column
//! starts.
//!
//! Split from [`super::measure`] for the line cap, along the line between a
//! height question and a width one.
use super::duedate;
use super::item::TodoItem;
pub(crate) use super::measure::*;
pub(crate) use super::render::*;
use crew_render::CellView;

/// Mirror of [`place_right`]'s arithmetic without the cells: the next free
/// slot after a width-`w` chip ending at `end` (or `end` unchanged when the
/// chip would reach the title zone and goes unplaced).
pub(crate) fn place_w(end: u16, w: u16) -> u16 {
    let start = end.saturating_sub(w);
    if start <= TITLE_COL {
        end
    } else {
        start.saturating_sub(2)
    }
}

/// Column the title's first line stops before when the chips ride beside it:
/// where the right-side chips (due, `@tag`, `✗`; in the done view the tick
/// time instead of the due) begin, minus the same two-column gap the chips
/// keep between each other.
///
/// Two, not one. At one the title and the chip beside it read as one phrase
/// the moment a title happens to fill its budget — `…and reverts @crew` — and
/// every other gap on the row was already two, so the one place it mattered
/// was the tightest.
pub(crate) fn inline_max(it: &TodoItem, cols: u16, now_ms: u64, done_view: bool) -> u16 {
    let mut right = del_col(cols).saturating_sub(2);
    if done_view {
        if it.done_ms.is_some() {
            right = place_w(right, 5); // "HH:MM"
        }
    } else if let Some(due) = it.due_ms {
        let lbl = duedate::label(due, it.due_has_time, now_ms);
        right = place_w(right, crate::chatwidth::str_w(&lbl) as u16);
    }
    if let Some(tag) = &it.project {
        right = place_w(right, crate::chatwidth::str_w(&format!("@{tag}")) as u16);
    }
    right
}

/// Whether the item carries anything on its right side at all.
pub(crate) fn has_chips(it: &TodoItem, done_view: bool) -> bool {
    it.project.is_some()
        || if done_view {
            it.done_ms.is_some()
        } else {
            it.due_ms.is_some()
        }
}

/// Most of a first line the row will fight for before it STACKS — chips off
/// the title line and onto a row of their own beneath it.
///
/// A narrow tile is where a right-aligned column stops being a column: the
/// chips are laid out first and take what they need, so on a 36-cell pane
/// `ship the release notes` was left three cells and hard-broke into `shi` /
/// `p the release notes`. Past that the row gives up on sharing the line and
/// becomes two bands, which is what it already looks like.
pub(crate) const MIN_TITLE_W: u16 = 20;

/// Whether this item stacks. Measured against the title it actually has, not
/// only [`MIN_TITLE_W`]: `pay rent` beside a tag and a due on a 40-cell pane
/// has eight cells of title and sixteen to put them in, and moving that down
/// a row would buy nothing. Never for an item with nothing on its right —
/// there is nothing to move down.
pub(crate) fn stacked(it: &TodoItem, cols: u16, now_ms: u64, done_view: bool) -> bool {
    if !has_chips(it, done_view) {
        return false;
    }
    let budget = inline_max(it, cols, now_ms, done_view).saturating_sub(TITLE_COL);
    let want = (crate::chatwidth::str_w(&it.title) as u16).min(MIN_TITLE_W);
    budget < want
}

/// The title's wrapped lines as char-index ranges: greedy word wrap, the
/// first line stopping where the chips begin, continuation lines spanning
/// the pane. Always at least one range, so every item owns a row.
pub(crate) fn title_lines(
    it: &TodoItem,
    cols: u16,
    now_ms: u64,
    done_view: bool,
) -> Vec<(usize, usize)> {
    let chars: Vec<char> = it.title.chars().collect();
    let wc = (cols.saturating_sub(2 + TITLE_COL)).max(1) as usize;
    let w0 = if stacked(it, cols, now_ms, done_view) {
        wc
    } else {
        (inline_max(it, cols, now_ms, done_view).saturating_sub(TITLE_COL)).max(1) as usize
    };
    wrap_ranges(&chars, w0, wc)
}

/// Greedy word wrap over `chars` into (start, end) char ranges: the first
/// line `w0` cells wide, continuations `wc`. Always at least one range.
pub(super) fn wrap_ranges(chars: &[char], w0: usize, wc: usize) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let mut start = 0;
    loop {
        let budget = if lines.is_empty() { w0 } else { wc };
        let fit = crate::chatwidth::fit_end(chars, start, budget);
        if fit >= chars.len() {
            lines.push((start, chars.len()));
            return lines;
        }
        // Break on the last space inside the window when the cut would land
        // mid-word; a single over-long word hard-breaks.
        let cut = chars[start..fit]
            .iter()
            .rposition(|c| c.is_whitespace())
            .map(|i| start + i)
            .filter(|&i| i > start && !chars[fit].is_whitespace())
            .unwrap_or(fit);
        lines.push((start, cut));
        start = cut;
        while start < chars.len() && chars[start].is_whitespace() {
            start += 1;
        }
        if start >= chars.len() {
            return lines;
        }
    }
}

/// Place `s` ending at `end` (exclusive of the following gap) on `row`;
/// returns the column two left of where it started (the next slot).
pub(crate) fn place_right(
    out: &mut Vec<CellView>,
    s: &str,
    end: u16,
    row: u16,
    fg: (u8, u8, u8),
    bold: bool,
) -> u16 {
    let w = crate::chatwidth::str_w(s) as u16;
    let start = end.saturating_sub(w);
    if start <= TITLE_COL {
        return end;
    }
    let styled = s.chars().map(|c| (c, ()));
    crate::chatwidth::place_row(start, end, styled, |x, c, ()| {
        out.push(cell(x, row, c, fg, bold))
    });
    start.saturating_sub(2)
}
