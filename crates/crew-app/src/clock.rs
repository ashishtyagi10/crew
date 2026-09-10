//! Sidebar clock section: a `TIME` divider above the local time and date.
use crew_render::CellView;

use crate::boxdraw::section_header;

use crate::palette::accent;

/// Rows the clock section occupies, including a one-row gap below it.
pub const CLOCK_H: u16 = 4;

/// Current local `(time, date)` as display strings, e.g. `("14:03:09", "Sat 21 Jun")`.
pub fn now_strings() -> (String, String) {
    let now = chrono::Local::now();
    (
        now.format("%H:%M:%S").to_string(),
        now.format("%a %d %b").to_string(),
    )
}

/// Render the clock section: a `TIME` rule on row 0, `time` and `date` centered
/// on rows 1 and 2, and the weather `strip` (see `navweather`) on row 3 —
/// the gap row, which the strip is quiet enough to stand in for.
pub fn clock_cells(time: &str, date: &str, strip: Option<&str>, cols: u16) -> Vec<CellView> {
    if cols < 10 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut out = section_header("TIME", cols, t.border_normal, accent(), t.page_bg);
    put_centered(&mut out, time, 1, cols, accent(), true, t.page_bg);
    put_centered(&mut out, date, 2, cols, t.ink, false, t.page_bg);
    if let Some(s) = strip {
        let s = crate::chatwidth::clip_w(s, usize::from(cols.saturating_sub(2)));
        put_centered(&mut out, &s, 3, cols, t.text_muted, false, t.page_bg);
    }
    out
}

fn put_centered(
    out: &mut Vec<CellView>,
    s: &str,
    row: u16,
    cols: u16,
    fg: (u8, u8, u8),
    bold: bool,
    bg: (u8, u8, u8),
) {
    // Display columns, not chars, and one column of air at the right edge —
    // the weather strip carries glyphs the other nav rows already measure.
    let w = crate::chatwidth::str_w(s) as u16;
    let start = if w < cols { (cols - w) / 2 } else { 0 };
    let styled = s.chars().map(|c| (c, ()));
    crate::chatwidth::place_row(start, cols.saturating_sub(1), styled, |col, c, ()| {
        out.push(CellView {
            col,
            row,
            c,
            fg,
            bg,
            bold,
            ..Default::default()
        });
    });
}

#[cfg(test)]
#[path = "clock_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "clockstrip_tests.rs"]
mod strip_tests;
