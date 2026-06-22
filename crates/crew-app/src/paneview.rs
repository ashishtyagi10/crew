//! Rendering panes to `PaneScene`s, including the corner badges: index (for
//! Cmd+1..9 / Ctrl+Tab nav), scrollback, and activity.
use crew_render::{CellView, PaneScene};

use crate::pane::{Pane, PaneContent};

/// Accent green for the focused pane's badge; muted grey otherwise.
const BADGE_ON: (u8, u8, u8) = (0, 255, 160);
const BADGE_OFF: (u8, u8, u8) = (110, 110, 120);
/// Amber for the "viewing scrollback" indicator.
const SCROLL_HINT: (u8, u8, u8) = (230, 180, 90);
/// Cyan dot marking a non-focused pane with new output.
const ACTIVITY: (u8, u8, u8) = (120, 200, 255);

fn corner(cells: &mut Vec<CellView>, col: u16, c: char, fg: (u8, u8, u8)) {
    cells.push(CellView {
        col,
        row: 0,
        c,
        fg,
        bg: (0, 0, 0),
        bold: true,
        italic: false,
    });
}

/// Single-digit index badge in the top-right corner (panes 1-9 only).
fn add_badge(cells: &mut Vec<CellView>, n: usize, cols: u16, focused: bool) {
    if cols < 3 || !(1..=9).contains(&n) {
        return;
    }
    let c = char::from_digit(n as u32, 10).unwrap_or('?');
    corner(
        cells,
        cols - 2,
        c,
        if focused { BADGE_ON } else { BADGE_OFF },
    );
}

/// Arrow marking a pane that's scrolled away from the live bottom.
fn add_scroll_badge(cells: &mut Vec<CellView>, cols: u16) {
    if cols < 6 {
        return;
    }
    corner(cells, cols - 4, '⇡', SCROLL_HINT);
}

/// Dot marking a non-focused pane that produced output since you last saw it.
fn add_activity_badge(cells: &mut Vec<CellView>, cols: u16) {
    if cols < 8 {
        return;
    }
    corner(cells, cols - 6, '●', ACTIVITY);
}

/// Build a `Vec<PaneScene>` from the current pane state (for `renderer.frame`).
/// Each pane gets corner badges (index when >1 pane, scrollback, activity).
pub fn build_scenes(panes: &[Pane], focused: Option<usize>) -> Vec<PaneScene> {
    let multi = panes.len() > 1;
    panes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut cells = p.cells();
            if multi {
                add_badge(&mut cells, i + 1, p.grid.cols, focused == Some(i));
            }
            if let PaneContent::Terminal(t) = &p.content {
                if t.pty.display_offset() > 0 {
                    add_scroll_badge(&mut cells, p.grid.cols);
                }
            }
            if p.activity && focused != Some(i) {
                add_activity_badge(&mut cells, p.grid.cols);
            }
            PaneScene {
                cells,
                x: p.rect.x,
                y: p.rect.y,
                w: p.rect.w,
                h: p.rect.h,
                focused: focused == Some(i),
                bordered: true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_badge_in_top_right() {
        let mut cells = Vec::new();
        add_badge(&mut cells, 3, 40, true);
        assert!(cells
            .iter()
            .any(|c| c.c == '3' && c.row == 0 && c.col == 38 && c.fg == BADGE_ON));
    }

    #[test]
    fn no_index_badge_out_of_range_or_too_narrow() {
        let mut cells = Vec::new();
        add_badge(&mut cells, 0, 40, false);
        add_badge(&mut cells, 12, 40, false);
        add_badge(&mut cells, 2, 2, false);
        assert!(cells.is_empty());
    }

    #[test]
    fn scroll_and_activity_badges() {
        let mut cells = Vec::new();
        add_scroll_badge(&mut cells, 40);
        add_activity_badge(&mut cells, 40);
        assert!(cells.iter().any(|c| c.c == '⇡' && c.fg == SCROLL_HINT));
        assert!(cells.iter().any(|c| c.c == '●' && c.fg == ACTIVITY));
    }
}
