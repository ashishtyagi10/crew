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

/// Render the load section: a `LOAD 1·5·15m` rule on row 0 and as many of the
/// three averages as the nav is wide enough to draw whole on row 1, coloured
/// by the 1-minute load relative to `cores`.
pub fn load_cells(one: f64, five: f64, fifteen: f64, cores: f64, cols: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let fg = load_color(one, cores);
    let past = crew_theme::readable::secondary(fg, t.page_bg);
    // Longest first. A narrow nav gives up the oldest average whole rather
    // than cutting the last one in half — `3.` used to be what the docked nav
    // showed at the narrow end of the resize range, and `3.` is not a smaller
    // reading than `3.64`, it is a wrong one.
    let ladder = [
        format!("{one:.2}  {five:.2}  {fifteen:.2}"),
        format!("{one:.2} {five:.2} {fifteen:.2}"),
        format!("{one:.2} {five:.2}"),
        format!("{one:.2}"),
    ];
    let refs: Vec<&str> = ladder.iter().map(String::as_str).collect();
    let shown = crate::navtext::fit(&refs, cols);
    // The rule's key names exactly the averages that survived: a key claiming
    // three when two are drawn is worse than no key at all.
    let key = match shown.split_whitespace().count() {
        0..=1 => "1m",
        2 => "1·5m",
        _ => "1·5·15m",
    };
    let mut out = crate::boxdraw::section_header_key(
        "LOAD",
        key,
        cols,
        t.border_normal,
        accent(),
        t.dim,
        t.page_bg,
    );
    // One colour, three ranks: the 1-minute figure is the only one that says
    // anything about *now*, so it keeps the load colour at full strength and
    // the history steps back. Separate hues would say they measured different
    // things; `text_muted` would throw away the warning they share.
    // The separator the fitted rung chose, so the coloured runs land exactly
    // where the string put them rather than on a spacing rule of their own.
    let sep = if shown.contains("  ") { 2 } else { 1 };
    let mut col = crate::navtext::INDENT;
    for (i, word) in shown.split_whitespace().enumerate() {
        crate::navtext::put_at(
            &mut out,
            word,
            col,
            1,
            cols.saturating_sub(1),
            if i == 0 { fg } else { past },
        );
        col += word.chars().count() as u16 + sep;
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

    /// Lay cells on `row` back out by column, so the gaps a section left
    /// between runs are in the string too.
    fn row_text(cells: &[CellView], row: u16) -> String {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
        v.sort_by_key(|c| c.col);
        let Some(first) = v.first() else {
            return String::new();
        };
        let mut out = String::new();
        let mut at = first.col;
        for c in v {
            for _ in at..c.col {
                out.push(' ');
            }
            out.push(c.c);
            at = c.col + 1;
        }
        out
    }

    /// A narrow nav gives up the oldest average whole. `3.` is not a smaller
    /// reading than `3.64`; it is a wrong one, and it is what the docked nav
    /// showed at the narrow end of the resize range.
    #[test]
    fn a_narrow_nav_drops_an_average_rather_than_half_of_one() {
        let _g = crate::app::theme_test_guard();
        let row = |cols| row_text(&load_cells(4.51, 4.20, 3.64, 8.0, cols), 1);
        assert_eq!(row(24), "4.51  4.20  3.64");
        assert_eq!(row(19), "4.51 4.20 3.64");
        assert_eq!(row(17), "4.51 4.20");
        assert_eq!(row(12), "4.51");
        for cols in 10..40u16 {
            let r = row(cols);
            assert!(!r.ends_with('.'), "{cols}: half a number: {r:?}");
            assert!(!r.contains('\u{2026}'), "{cols}: clipped: {r:?}");
        }
    }

    /// …and the rule's key names exactly the averages that survived. A key
    /// claiming three when two are drawn is worse than no key at all.
    #[test]
    fn the_key_names_only_the_averages_that_are_drawn() {
        let _g = crate::app::theme_test_guard();
        let key = |cols| -> String {
            let cells: Vec<_> = load_cells(4.51, 4.20, 3.64, 8.0, cols)
                .into_iter()
                .filter(|c| c.c != '\u{2500}')
                .collect();
            row_text(&cells, 0).trim().to_string()
        };
        assert_eq!(key(30), "LOAD 1·5·15m");
        assert_eq!(key(18), "LOAD 1·5·15m", "three values, three named");
        assert_eq!(key(15), "LOAD 1·5m", "two values, two named");
        // Narrower than the rule can carry a key at all: the title stays, the
        // key is what goes.
        assert_eq!(key(12), "LOAD");
    }

    /// One colour, three ranks: the 1-minute figure is the only one that says
    /// anything about now, so it alone carries the load colour at full
    /// strength. On a page with headroom that is a visible difference.
    #[test]
    fn only_the_one_minute_figure_carries_the_load_colour() {
        let _g = crate::app::theme_test_guard();
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
        let t = crew_theme::theme();
        let cells = load_cells(9.0, 8.0, 7.0, 4.0, 26); // oversubscribed: red
        let at = |col: u16| cells.iter().find(|c| c.row == 1 && c.col == col);
        assert_eq!(at(3).map(|c| c.fg), Some(red()));
        let past = crew_theme::readable::secondary(red(), t.page_bg);
        assert_eq!(at(9).map(|c| c.fg), Some(past), "the 5-minute steps back");
        assert_ne!(past, red(), "and on this page that is a real difference");
    }
}
