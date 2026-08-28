//! Sidebar LOG section: a live, scrolling tail of recent status messages (the
//! same lines flashed on the input bar). Unlike the 3-second flash, the log
//! keeps recent activity visible in its own left-nav section — newest at the
//! bottom, so the latest line sits nearest the pane list below it.
use crew_render::CellView;

use crate::applog::{LogEntry, LogLevel};
use crate::boxdraw::section_header;

use crate::palette::accent;

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
        let (stamp, msg) = split_stamp(&e.text);
        // The stamp is fixed furniture on every line — same six columns, same
        // shape — so it is dimmed out of the way and the message keeps the
        // ink. What is left after it is the message's clip budget, and the
        // clip is the same `…` every card legend uses: a nav two columns too
        // narrow used to end a line mid-word ("updated to cr"), which reads
        // as a bug rather than as a line that did not fit.
        let max_col = cols.saturating_sub(1);
        let room = max_col.saturating_sub(TEXT_COL) as usize;
        let stamp_w = crate::chatwidth::str_w(stamp).min(room);
        let body = crate::chatwidth::clip_w(msg, room - stamp_w);
        let styled = stamp
            .chars()
            .map(|c| (c, t.dim))
            .chain(body.chars().map(|c| (c, fg)));
        crate::chatwidth::place_row(TEXT_COL, max_col, styled, |x, c, fg| {
            out.push(CellView {
                col: x,
                row: 1 + k as u16,
                c,
                fg,
                bg: t.page_bg,
                ..Default::default()
            });
        });
    }
    out
}

/// Column the entry text starts on, under the `LOG` rule's own indent.
const TEXT_COL: u16 = 2;

/// Split a buffered entry into its `HH:MM ` stamp and the message. The stamp
/// is prepended when the line is buffered, so it is a prefix of the text
/// rather than a field — recognised by shape, and absent (`""`) on any line
/// that does not carry one.
fn split_stamp(s: &str) -> (&str, &str) {
    let b = s.as_bytes();
    let stamped = b.len() > 6
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2] == b':'
        && b[3].is_ascii_digit()
        && b[4].is_ascii_digit()
        && b[5] == b' ';
    if stamped {
        s.split_at(6)
    } else {
        ("", s)
    }
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
            ..Default::default()
        });
    });
}

#[cfg(test)]
#[path = "navlog_tests.rs"]
mod tests;
