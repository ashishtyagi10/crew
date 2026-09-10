//! The standing sign hint mode owes: a mode that eats the next keystroke
//! has to say what the keys do, on the pane, for as long as it lasts.
//!
//! `Cmd+E` labelled every URL, path and hash and washed the pane, and
//! nothing on screen said that a letter copies, its capital opens, and Esc
//! leaves — the rule was in `/keys` and nowhere you were looking. One
//! muted row on the pane's last line carries it, chosen by width rather
//! than cut, the way the composer's own legend and `/disk`'s hint are.
use crew_render::CellView;

/// The forms, longest first; every one names the way out.
pub(crate) const FORMS: &[&str] = &[
    "a copies \u{b7} A opens \u{b7} esc cancels",
    "a copies \u{b7} A opens \u{b7} esc",
    "a copy \u{b7} A open \u{b7} esc",
    "esc",
];

/// The widest form that fits `cols` from column 1 with one column of air.
pub(crate) fn text(cols: u16) -> &'static str {
    let room = usize::from(cols.saturating_sub(2));
    FORMS
        .iter()
        .find(|s| crate::chatwidth::str_w(s) <= room)
        .or(FORMS.last())
        .copied()
        .unwrap_or("")
}

/// Label the pane if hint mode is on it ([`crate::hints::mark_pane`]) and,
/// if so, stamp the legend over its last row — the row a modal may claim,
/// since the mode ends before anything under it is wanted again.
pub(crate) fn mark(cells: &mut Vec<CellView>, pane: usize, cols: u16, rows: u16) {
    if crate::hints::mark_pane(cells, pane) {
        stamp(cells, cols, rows);
    }
}

/// The legend over the last of `rows`, replacing whatever was there.
pub(crate) fn stamp(cells: &mut Vec<CellView>, cols: u16, rows: u16) {
    if rows < 2 || cols < 4 {
        return;
    }
    let t = crew_theme::theme();
    let row = rows - 1;
    cells.retain(|c| c.row != row);
    let styled = text(cols).chars().map(|c| (c, t.text_muted));
    crate::chatwidth::place_row(1, cols, styled, |x, c, fg| {
        cells.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg: t.page_bg,
            ..Default::default()
        })
    });
}

#[cfg(test)]
#[path = "hintlegend_tests.rs"]
mod tests;
