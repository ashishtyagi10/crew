//! Sidebar load section: a `LOAD` divider above the 1/5/15-minute system load
//! average, coloured by load-per-core (green / amber / red). Complements the
//! instantaneous SYSTEM gauges with a sense of sustained pressure.
use crew_render::CellView;

use crate::palette::accent;
/// The two warning colours, darkened until the page can carry them. They
/// shipped as the flat constants `(230, 180, 90)` and `(230, 90, 90)`, which
/// on a light theme read at 1.7 and 3.2 — a warning colour at 1.7 is a warning
/// nobody receives.
fn amber() -> (u8, u8, u8) {
    crew_theme::readable::warn(crew_theme::theme())
}

fn red() -> (u8, u8, u8) {
    crew_theme::readable::danger(crew_theme::theme())
}

/// Current `(one, five, fifteen)`-minute load averages (0.0 where unsupported).
pub fn load_avg() -> (f64, f64, f64) {
    let l = sysinfo::System::load_average();
    (l.one, l.five, l.fifteen)
}

/// Logical-core count, used to scale the load colour (never zero).
pub fn cores() -> f64 {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .max(1) as f64
}

/// Colour for a 1-minute load over `cores`: green when comfortably under one
/// task per core, amber approaching saturation, red once oversubscribed.
fn load_color(one: f64, cores: f64) -> (u8, u8, u8) {
    let per_core = one / cores;
    if per_core < 0.7 {
        accent()
    } else if per_core < 1.0 {
        amber()
    } else {
        red()
    }
}

/// Render the load section: a `LOAD 1·5·15m` rule on row 0 and the three
/// averages on row 1, coloured by the 1-minute load relative to `cores`.
///
/// The three are the same measurement at three ages, so they keep one colour
/// and differ by rank instead: the 1-minute figure — the only one that says
/// anything about *now* — carries the load colour at full strength, and the
/// two history figures step back with [`crew_theme::readable::secondary`].
/// Giving them different hues would say they measured different things;
/// dropping them to `text_muted` would throw away the warning they share.
pub fn load_cells(one: f64, five: f64, fifteen: f64, cores: f64, cols: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = crate::boxdraw::section_header_key(
        "LOAD",
        "1·5·15m",
        cols,
        t.border_normal,
        accent(),
        t.dim,
        t.page_bg,
    );
    let fg = load_color(one, cores);
    let past = crew_theme::readable::secondary(fg, t.page_bg);
    let max = cols.saturating_sub(4) as usize;
    let mut col = 3usize;
    for (i, (v, fg)) in [(one, fg), (five, past), (fifteen, past)]
        .into_iter()
        .enumerate()
    {
        if i > 0 {
            col += 2;
        }
        for c in format!("{v:.2}").chars() {
            if col >= 3 + max {
                return out;
            }
            out.push(CellView {
                col: col as u16,
                row: 1,
                c,
                fg,
                bg: t.page_bg,
                bold: false,
                italic: false,
                ..Default::default()
            });
            col += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_color_thresholds() {
        // The colours are derived from the live theme now, so two reads of
        // the process global must not straddle another test switching it.
        let _g = crate::app::theme_test_guard();
        // 4 cores: 1.0 load → 0.25/core (green); 3.0 → 0.75 (amber); 5.0 → 1.25 (red)
        assert_eq!(load_color(1.0, 4.0), accent());
        assert_eq!(load_color(3.0, 4.0), amber());
        assert_eq!(load_color(5.0, 4.0), red());
    }

    #[test]
    fn load_cells_render_divider_and_numbers() {
        let cells = load_cells(1.5, 0.8, 0.5, 4.0, 24);
        // LOAD divider on row 0
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        assert!(cells.iter().any(|c| c.c == 'L' && c.row == 0));
        // formatted 1-minute value present on row 1
        let row1: String = {
            let mut cs: Vec<_> = cells.iter().filter(|c| c.row == 1).collect();
            cs.sort_by_key(|c| c.col);
            cs.iter().map(|c| c.c).collect()
        };
        assert!(row1.contains("1.50"));
        assert!(row1.contains("0.50"));
    }

    #[test]
    fn cores_is_at_least_one() {
        assert!(cores() >= 1.0);
    }
}
