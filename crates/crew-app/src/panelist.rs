//! Sidebar PANES section: a live list of open panes (index, name/title, a `▸`
//! focus marker, and an activity dot) so the whole grid is visible at a glance —
//! handy when a single pane is zoomed.
use crew_render::CellView;

use crate::boxdraw::section_header;

use crate::palette::accent;
const TITLE_ON: (u8, u8, u8) = (225, 225, 225);
const TITLE_OFF: (u8, u8, u8) = (150, 150, 160);
const ACTIVITY: (u8, u8, u8) = (120, 200, 255);
const BORDER: (u8, u8, u8) = (110, 110, 120);
const BG: (u8, u8, u8) = (0, 0, 0);

/// One row of the PANES list.
pub struct PaneRow {
    pub index: usize,
    pub title: String,
    pub focused: bool,
    pub activity: bool,
}

/// Render the PANES section: a `PANES` rule on row 0, then one row per pane
/// (up to `limit`) beneath it.
pub fn pane_cells(panes: &[PaneRow], cols: u16, limit: usize) -> Vec<CellView> {
    let mut out = section_header("PANES", cols, BORDER, accent(), BG);
    for (k, p) in panes.iter().take(limit).enumerate() {
        let row = 1 + k as u16;
        let head = format!("{} {}", if p.focused { '▸' } else { ' ' }, p.index);
        let head_fg = if p.focused { accent() } else { TITLE_OFF };
        write(&mut out, &head, 2, row, head_fg, cols - 1);
        let tstart = 2 + head.chars().count() as u16 + 1;
        let title_fg = if p.focused { TITLE_ON } else { TITLE_OFF };
        write(
            &mut out,
            &p.title,
            tstart,
            row,
            title_fg,
            cols.saturating_sub(3),
        );
        if p.activity {
            write(&mut out, "●", cols.saturating_sub(2), row, ACTIVITY, cols);
        }
    }
    out
}

/// Write `s` at `(col, row)`, stopping before `max_col`.
fn write(out: &mut Vec<CellView>, s: &str, col: u16, row: u16, fg: (u8, u8, u8), max_col: u16) {
    for (i, c) in s.chars().enumerate() {
        let x = col + i as u16;
        if x >= max_col {
            break;
        }
        out.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(index: usize, title: &str, focused: bool, activity: bool) -> PaneRow {
        PaneRow {
            index,
            title: title.into(),
            focused,
            activity,
        }
    }

    #[test]
    fn pane_cells_lists_focus_and_activity() {
        let panes = [row(1, "build", true, false), row(2, "server", false, true)];
        let cells = pane_cells(&panes, 24, 10);
        // PANES rule on row 0
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        assert!(cells.iter().any(|c| c.c == 'P' && c.row == 0));
        // focus marker + title for the focused pane on row 1
        assert!(cells.iter().any(|c| c.c == '▸' && c.row == 1));
        assert!(cells
            .iter()
            .any(|c| c.c == 'b' && c.row == 1 && c.fg == TITLE_ON));
        // the unfocused pane's title is dimmed on row 2, with an activity dot
        assert!(cells
            .iter()
            .any(|c| c.c == 's' && c.row == 2 && c.fg == TITLE_OFF));
        assert!(cells
            .iter()
            .any(|c| c.c == '●' && c.row == 2 && c.fg == ACTIVITY));
    }

    #[test]
    fn pane_cells_respects_limit() {
        let panes: Vec<PaneRow> = (1..=5).map(|i| row(i, "x", false, false)).collect();
        let cells = pane_cells(&panes, 24, 2);
        // only two pane rows (1 and 2) are drawn; nothing reaches row 3
        assert!(!cells.iter().any(|c| c.row == 3));
    }
}
