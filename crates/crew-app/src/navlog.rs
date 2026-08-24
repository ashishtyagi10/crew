//! Sidebar LOG section: a live, scrolling tail of recent status messages (the
//! same lines flashed on the input bar). Unlike the 3-second flash, the log
//! keeps recent activity visible in its own left-nav section — newest at the
//! bottom, so the latest line sits nearest the pane list below it.
use crew_render::CellView;

use crate::applog::{LogEntry, LogLevel};
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

/// The window of `entries` a LOG scrolled `back` lines shows in `max_lines`
/// rows: `(start, shown)`. `back` is clamped to what there is, so a wheel
/// spun past the oldest entry stops on it instead of scrolling into nothing.
pub fn window(n: usize, max_lines: usize, back: usize) -> (usize, usize) {
    let shown = n.min(max_lines);
    let back = back.min(n - shown);
    (n - shown - back, shown)
}

/// The furthest a LOG of `n` entries can be scrolled back in `max_lines` rows.
pub fn max_back(n: usize, max_lines: usize) -> usize {
    n.saturating_sub(max_lines)
}

/// Render the LOG section: a `LOG` rule on row 0, then `max_lines` entries
/// beneath it (oldest first, newest on the bottom row), ending `back` entries
/// before the newest. Error-level entries render in the bell (attention) color
/// so failures stand out of the muted activity tail. Empty when there are no
/// entries, no room, or the card is too narrow.
pub fn log_cells(entries: &[LogEntry], cols: u16, max_lines: usize, back: usize) -> Vec<CellView> {
    if entries.is_empty() || max_lines == 0 || cols < 4 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut out = section_header("LOG", cols, t.border_normal, accent(), t.page_bg);
    let (start, shown) = window(entries.len(), max_lines, back);
    // Scrolled back, the rule says so — the tail is no longer live, and a log
    // that silently stops following looks like a log that stopped.
    if back > 0 {
        let mark = format!("\u{21e1}{back}");
        write(
            &mut out,
            &mark,
            cols.saturating_sub(mark.chars().count() as u16 + 1),
            0,
            t.status_fg,
            cols,
            t.page_bg,
        );
    }
    for (k, e) in entries[start..start + shown].iter().enumerate() {
        let fg = match e.level {
            LogLevel::Info => t.text_muted,
            LogLevel::Error => t.bell,
        };
        write(
            &mut out,
            &e.text,
            2,
            1 + k as u16,
            fg,
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
#[path = "navlog_tests.rs"]
mod tests;
