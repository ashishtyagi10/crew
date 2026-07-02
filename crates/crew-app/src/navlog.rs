//! Sidebar LOG section: a live, scrolling tail of recent status messages (the
//! same lines flashed on the input bar). Unlike the 3-second flash, the log
//! keeps recent activity visible in its own left-nav section — newest at the
//! bottom, so the latest line sits nearest the pane list below it.
use crew_render::CellView;

use crate::boxdraw::section_header;

use crate::palette::accent;

/// Most recent log entries shown in the LOG section (older ones scroll off).
pub const LOG_LINES: usize = 5;

/// Rows the LOG section occupies for `n` buffered entries: a rule, up to
/// [`LOG_LINES`] entry rows, and a one-row gap — or 0 when the log is empty.
/// The sidebar uses this to reserve the block and keep hit-testing aligned.
pub fn log_block(n: usize) -> u16 {
    if n == 0 {
        0
    } else {
        n.min(LOG_LINES) as u16 + 2
    }
}

/// Render the LOG section: a `LOG` rule on row 0, then the most recent
/// `max_lines` entries beneath it (oldest first, newest on the bottom row).
/// Empty when there are no entries, no room, or the card is too narrow.
pub fn log_cells(entries: &[String], cols: u16, max_lines: usize) -> Vec<CellView> {
    if entries.is_empty() || max_lines == 0 || cols < 4 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut out = section_header("LOG", cols, t.border_normal, accent(), t.page_bg);
    let shown = entries.len().min(max_lines);
    let start = entries.len() - shown;
    for (k, e) in entries[start..].iter().enumerate() {
        write(
            &mut out,
            e,
            2,
            1 + k as u16,
            t.text_muted,
            cols.saturating_sub(1),
            t.page_bg,
        );
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

    #[test]
    fn log_section_has_rule_and_newest_last() {
        let entries = ["first".to_string(), "second".to_string()];
        let cells = log_cells(&entries, 24, 5);
        // LOG rule + legend on row 0
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        assert!(cells.iter().any(|c| c.c == 'L' && c.row == 0));
        // oldest entry on row 1, newest on row 2 (bottom)
        assert!(cells.iter().any(|c| c.c == 'f' && c.row == 1));
        assert!(cells.iter().any(|c| c.c == 's' && c.row == 2));
    }

    #[test]
    fn log_keeps_only_the_most_recent_lines() {
        let entries: Vec<String> = (0..10).map(|i| format!("line{i}")).collect();
        let cells = log_cells(&entries, 24, 3);
        // only the last 3 entries are drawn (rows 1..=3); nothing on row 4
        assert!(!cells.iter().any(|c| c.row == 4));
        // the oldest shown is line7 (10 entries, last 3) — its '7' is on row 1
        assert!(cells.iter().any(|c| c.c == '7' && c.row == 1));
    }

    #[test]
    fn empty_or_narrow_renders_nothing() {
        assert!(log_cells(&[], 24, 5).is_empty());
        assert!(log_cells(&["x".to_string()], 24, 0).is_empty());
        assert!(log_cells(&["x".to_string()], 3, 5).is_empty());
    }
}
