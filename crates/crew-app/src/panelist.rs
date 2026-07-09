//! Sidebar PANES section: a live list of open panes (index, name/title, a `▸`
//! focus marker, and an activity dot) so the whole grid is visible at a glance —
//! handy when a single pane is zoomed.
use crew_render::CellView;

use crate::boxdraw::section_header;

use crate::palette::accent;

/// One row of the PANES list.
pub struct PaneRow {
    pub index: usize,
    pub title: String,
    pub focused: bool,
    pub activity: bool,
    /// Not visible in the content area — minimized into the nav (the pane's
    /// `[-]` border button) or covered while another pane is zoomed: drawn with
    /// a right-aligned `[+]`; clicking the row focuses the pane, which brings
    /// it back on screen.
    pub minimized: bool,
    /// A raised "needs you" marker: `(glyph, visible)`. The glyph names the
    /// event (`!` bell · `⚑` pattern · `✓` command done); `visible` is the
    /// blink phase (false hides the marker mid-pulse, the tint stays). Drawn
    /// in the bell colour, superseding the quiet activity dot.
    pub attention: Option<(char, bool)>,
}

/// Render the PANES section: a `PANES` rule on row 0, then one row per pane
/// (up to `limit`) beneath it.
pub fn pane_cells(panes: &[PaneRow], cols: u16, limit: usize) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = section_header("PANES", cols, t.border_normal, accent(), t.page_bg);
    for (k, p) in panes.iter().take(limit).enumerate() {
        let row = 1 + k as u16;
        let head = format!("{} {}", if p.focused { '▸' } else { ' ' }, p.index);
        let head_fg = if p.focused { accent() } else { t.text_muted };
        write(&mut out, &head, 2, row, head_fg, cols - 1, t.page_bg);
        let tstart = 2 + head.chars().count() as u16 + 1;
        let title_fg = if p.focused {
            t.ink
        } else if p.attention.is_some() {
            t.bell
        } else {
            t.text_muted
        };
        // A minimized row carries a right-aligned [+] restore button (ending a
        // cell left of the activity-dot slot); its title stops short of it.
        let tmax = if p.minimized {
            cols.saturating_sub(8)
        } else {
            cols.saturating_sub(3)
        };
        write(&mut out, &p.title, tstart, row, title_fg, tmax, t.page_bg);
        if p.minimized {
            write(
                &mut out,
                "[+]",
                cols.saturating_sub(6),
                row,
                accent(),
                cols.saturating_sub(2),
                t.page_bg,
            );
        }
        // The attention marker owns the dot slot while raised; the quiet
        // activity dot returns once the pane has been looked at.
        if let Some((glyph, on)) = p.attention {
            if on {
                write(
                    &mut out,
                    &glyph.to_string(),
                    cols.saturating_sub(2),
                    row,
                    t.bell,
                    cols,
                    t.page_bg,
                );
            }
        } else if p.activity {
            write(
                &mut out,
                "●",
                cols.saturating_sub(2),
                row,
                t.activity,
                cols,
                t.page_bg,
            );
        }
    }
    out
}

/// Write `s` at `(col, row)`, stopping before `max_col`.
fn write(
    out: &mut Vec<CellView>,
    s: &str,
    col: u16,
    row: u16,
    fg: (u8, u8, u8),
    max_col: u16,
    bg: (u8, u8, u8),
) {
    // Width-aware: pane titles can carry emoji/CJK (OSC titles) — a wide
    // glyph advances two columns (see `chatwidth`).
    crate::chatwidth::place_row(col, max_col, s.chars().map(|c| (c, fg)), |x, c, fg| {
        out.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold: false,
            italic: false,
        });
    });
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
            minimized: false,
            attention: None,
        }
    }

    #[test]
    fn pane_cells_marks_minimized_panes_with_a_restore_button() {
        let panes = [
            row(1, "build", true, false),
            PaneRow {
                minimized: true,
                ..row(2, "server", false, false)
            },
        ];
        let cells = pane_cells(&panes, 24, 10);
        // The minimized pane's row carries a right-aligned [+] restore button
        // ending one cell left of the activity-dot slot: cols 18..=20.
        let at = |col: u16, row: u16| {
            cells
                .iter()
                .find(|c| c.row == row && c.col == col)
                .map(|c| c.c)
        };
        assert_eq!(at(18, 2), Some('['));
        assert_eq!(at(19, 2), Some('+'));
        assert_eq!(at(20, 2), Some(']'));
        // …and only on minimized rows.
        assert!(!cells.iter().any(|c| c.c == '+' && c.row == 1));
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
            .any(|c| c.c == 'b' && c.row == 1 && c.fg == crew_theme::theme().ink));
        // the unfocused pane's title is dimmed on row 2, with an activity dot
        assert!(cells
            .iter()
            .any(|c| c.c == 's' && c.row == 2 && c.fg == crew_theme::theme().text_muted));
        assert!(cells
            .iter()
            .any(|c| c.c == '●' && c.row == 2 && c.fg == crew_theme::theme().activity));
    }

    #[test]
    fn attention_row_draws_the_marker_and_tints_the_title() {
        let panes = [
            row(1, "build", true, false),
            PaneRow {
                attention: Some(('!', true)),
                ..row(2, "server", false, true)
            },
        ];
        let cells = pane_cells(&panes, 24, 10);
        let bell = crew_theme::theme().bell;
        // marker glyph in the dot slot, in the bell (needs-you) colour
        assert!(cells
            .iter()
            .any(|c| c.c == '!' && c.row == 2 && c.col == 22 && c.fg == bell));
        // the title is tinted too, so the row is findable at a glance
        assert!(cells
            .iter()
            .any(|c| c.c == 's' && c.row == 2 && c.fg == bell));
        // attention supersedes the quiet activity dot
        assert!(!cells.iter().any(|c| c.c == '●' && c.row == 2));
    }

    #[test]
    fn attention_blink_off_phase_hides_the_marker_but_keeps_the_tint() {
        let panes = [PaneRow {
            attention: Some(('!', false)),
            ..row(1, "server", false, false)
        }];
        let cells = pane_cells(&panes, 24, 10);
        let bell = crew_theme::theme().bell;
        assert!(!cells.iter().any(|c| c.c == '!' && c.row == 1));
        assert!(cells
            .iter()
            .any(|c| c.c == 's' && c.row == 1 && c.fg == bell));
    }

    #[test]
    fn pane_cells_respects_limit() {
        let panes: Vec<PaneRow> = (1..=5).map(|i| row(i, "x", false, false)).collect();
        let cells = pane_cells(&panes, 24, 2);
        // only two pane rows (1 and 2) are drawn; nothing reaches row 3
        assert!(!cells.iter().any(|c| c.row == 3));
    }
}
