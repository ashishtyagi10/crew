//! Pure rendering of the system-stats sidebar section: a header + spaced gauges.
use crew_render::CellView;

use crate::boxdraw;
use crate::palette::accent;
use crate::stats::Stats;

const HEADER: &str = "SYSTEM";

/// Empty-track colour: the theme's recessed border shade, so the track sits
/// back like an unfocused card edge — and stays in-palette on the monochrome
/// CRT phosphor themes instead of a fixed grey.
fn track_color() -> (u8, u8, u8) {
    crew_theme::theme().border_normal
}

/// Bar colour by load: accent when low, the theme's status amber past 70%,
/// its bright-red ANSI slot past 90% — every tier drawn from the active
/// palette so a phosphor theme's gauges glow in that phosphor's hues.
pub(crate) fn fill_color(frac: f32) -> (u8, u8, u8) {
    let t = crew_theme::theme();
    // The bands come from `shapecues::Tier` so the colour and the shape cue
    // beside it can never disagree about which band a reading is in.
    match crate::shapecues::Tier::of(frac) {
        crate::shapecues::Tier::Nominal => accent(),
        crate::shapecues::Tier::Warn => t.status_fg,
        crate::shapecues::Tier::Critical => t.ansi[9],
    }
}

/// One gauge row laid out within `cols`: `label | space | bar | NNN%`.
fn gauge_cells(label: &str, frac: f32, row: u16, cols: u16) -> Vec<CellView> {
    if cols == 0 {
        return Vec::new();
    }
    let cols = cols as usize;
    let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u16;
    let pct_str = format!("{pct:>3}%");
    let pct_len = pct_str.len();

    let t = crew_theme::theme();
    let label_chars: Vec<char> = label.chars().collect();
    let label_len = label_chars.len();
    let mut cells: Vec<CellView> = Vec::with_capacity(cols);

    for (i, &c) in label_chars.iter().enumerate() {
        if cells.len() >= cols {
            break;
        }
        cells.push(cell(i as u16, row, c, t.ink, t.page_bg));
    }
    // The tier mark rides in the label's trailing space, in the fill's own
    // colour: the band is said twice for anyone who needs it and costs no
    // column, since that space was always there (see `shapecues`).
    let mark = crate::shapecues::Tier::of(frac).mark();
    if cells.len() < cols {
        let (c, fg) = match mark {
            Some(m) => (m, fill_color(frac)),
            None => (' ', t.ink),
        };
        cells.push(cell(label_len as u16, row, c, fg, t.page_bg));
    }

    let used = cells.len();
    let bar_width = cols.saturating_sub(label_len + 1 + pct_len);
    let filled = (frac.clamp(0.0, 1.0) * bar_width as f32).round() as usize;
    let fill = fill_color(frac);
    for i in 0..bar_width {
        if cells.len() >= cols {
            break;
        }
        let (c, fg) = if i < filled {
            ('█', fill)
        } else {
            ('░', track_color())
        };
        cells.push(cell((used + i) as u16, row, c, fg, t.page_bg));
    }

    let pct_start = cols.saturating_sub(pct_len);
    for (i, c) in pct_str.chars().enumerate() {
        let col = pct_start + i;
        if col >= cols {
            break;
        }
        if col < cells.len() {
            cells[col] = cell(col as u16, row, c, t.ink, t.page_bg);
        } else {
            cells.push(cell(col as u16, row, c, t.ink, t.page_bg));
        }
    }
    cells
}

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg,
        bold: false,
        italic: false,
    }
}

/// Render the stats section: a `SYSTEM` rule on row 0 (fieldset-legend style)
/// with CPU/MEM/DISK gauges on rows 1/2/3 beneath it. Sidebar sections stack as
/// their own dividers below.
pub(crate) fn render_stats(stats: Stats, cols: u16, rows: u16) -> Vec<CellView> {
    let mut out = Vec::new();
    if cols < 8 || rows < 4 {
        return out;
    }
    let t = crew_theme::theme();
    out.extend(boxdraw::section_header(
        HEADER,
        cols,
        t.border_normal,
        accent(),
        t.page_bg,
    ));

    // Content indented to align under the section legend (col 3).
    let cstart = 3u16;
    let inner = cols.saturating_sub(cstart + 1);

    let gauges = [
        ("CPU ", stats.cpu),
        ("MEM ", stats.mem),
        ("DISK", stats.disk),
    ];
    for (i, (label, frac)) in gauges.into_iter().enumerate() {
        let row = 1 + i as u16;
        if row >= rows {
            break;
        }
        for mut g in gauge_cells(label, frac, 0, inner) {
            g.col += cstart;
            g.row = row;
            out.push(g);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_color_thresholds() {
        let t = crew_theme::theme();
        assert_eq!(fill_color(0.5), crate::palette::accent());
        assert_eq!(fill_color(0.8), t.status_fg);
        assert_eq!(fill_color(0.95), t.ansi[9]);
        assert_eq!(track_color(), t.border_normal);
    }

    #[test]
    fn gauge_50_pct_balanced() {
        let cells = gauge_cells("CPU ", 0.5, 0, 40);
        assert!(!cells.is_empty());
        let filled = cells.iter().filter(|c| c.c == '█').count();
        let track = cells.iter().filter(|c| c.c == '░').count();
        assert!((filled as i32 - track as i32).unsigned_abs() <= 1);
    }

    #[test]
    fn gauge_0_pct_no_filled() {
        let cells = gauge_cells("CPU ", 0.0, 0, 40);
        assert_eq!(cells.iter().filter(|c| c.c == '█').count(), 0);
    }

    #[test]
    fn gauge_100_pct_no_track() {
        let cells = gauge_cells("CPU ", 1.0, 0, 40);
        assert_eq!(cells.iter().filter(|c| c.c == '░').count(), 0);
    }

    #[test]
    fn render_stats_legend_and_gauges() {
        let stats = Stats {
            cpu: 0.1,
            mem: 0.2,
            disk: 0.3,
            ..Default::default()
        };
        let cells = render_stats(stats, 24, 12);
        // flat divider, not a box
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        assert!(!cells.iter().any(|c| matches!(c.c, '╭' | '╮' | '╰' | '╯')));
        // SYSTEM legend on the divider row
        assert!(cells.iter().any(|c| c.c == 'S' && c.row == 0));
        // gauge bars present, stacked on rows 1/2/3
        assert!(cells.iter().any(|c| c.c == '█' || c.c == '░'));
        let rows: std::collections::HashSet<u16> = cells.iter().map(|c| c.row).collect();
        assert!(rows.contains(&1) && rows.contains(&2) && rows.contains(&3));
    }

    /// The tier mark has to reach the drawn row, and cost nothing when it is
    /// not wanted: it rides the label's trailing space, so the bar and the
    /// percentage must land on exactly the same columns either way. A cue
    /// that shifted the layout would be a cue nobody could leave on.
    #[test]
    fn the_tier_mark_rides_the_label_space_without_moving_anything() {
        let _g = crate::app::motion_test_guard();
        let cols = 40;
        crate::shapecues::set(false);
        let off = gauge_cells("CPU ", 0.95, 0, cols);
        crate::shapecues::set(true);
        let on = gauge_cells("CPU ", 0.95, 0, cols);
        crate::shapecues::set(false);

        assert_eq!(off.len(), on.len(), "the cue must not change the width");
        let bar_and_pct = |v: &[CellView]| -> Vec<(u16, char)> {
            v.iter()
                .filter(|c| c.col > 4)
                .map(|c| (c.col, c.c))
                .collect()
        };
        assert_eq!(
            bar_and_pct(&off),
            bar_and_pct(&on),
            "the bar and the reading must not move"
        );

        let at = |v: &[CellView], col: u16| v.iter().find(|c| c.col == col).map(|c| c.c);
        assert_eq!(at(&off, 4), Some(' '), "off, the slot is the label space");
        assert_eq!(at(&on, 4), Some('\u{203c}'), "on, critical is marked");
    }

    /// Three bands, three appearances — a warning and a critical reading that
    /// mark the same are no better than two that only differ in colour.
    #[test]
    fn each_band_marks_differently_on_a_drawn_row() {
        let _g = crate::app::motion_test_guard();
        crate::shapecues::set(true);
        let mark = |frac: f32| {
            gauge_cells("CPU ", frac, 0, 40)
                .iter()
                .find(|c| c.col == 4)
                .map(|c| c.c)
        };
        let (n, w, c) = (mark(0.3), mark(0.8), mark(0.95));
        crate::shapecues::set(false);
        assert_eq!(n, Some(' '), "nominal stays quiet");
        assert_ne!(w, n);
        assert_ne!(c, n);
        assert_ne!(w, c);
    }
}
