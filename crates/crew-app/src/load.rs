//! Sidebar load section: a `LOAD` divider above the 1/5/15-minute system load
//! average, coloured by load-per-core (green / amber / red). Complements the
//! instantaneous SYSTEM gauges with a sense of sustained pressure.
use crew_render::CellView;

use crate::boxdraw::section_header;

use crate::palette::accent;
const AMBER: (u8, u8, u8) = (230, 180, 90);
const RED: (u8, u8, u8) = (230, 90, 90);
const DIM: (u8, u8, u8) = (150, 150, 160);
const BORDER: (u8, u8, u8) = (110, 110, 120);
const BG: (u8, u8, u8) = (0, 0, 0);

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
        AMBER
    } else {
        RED
    }
}

/// Render the load section: a `LOAD` rule on row 0 and the three averages on
/// row 1, the trio coloured by the 1-minute load relative to `cores`.
pub fn load_cells(one: f64, five: f64, fifteen: f64, cores: f64, cols: u16) -> Vec<CellView> {
    let mut out = section_header("LOAD", cols, BORDER, accent(), BG);
    let fg = load_color(one, cores);
    let nums = format!("{one:.2}  {five:.2}  {fifteen:.2}");
    let max = cols.saturating_sub(4) as usize;
    for (i, c) in nums.chars().take(max).enumerate() {
        out.push(CellView {
            col: 3 + i as u16,
            row: 1,
            c,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        });
    }
    // Trailing "1m 5m 15m" hint when the row has spare width.
    let hint = "1·5·15m";
    let hstart = nums.chars().count() + 5;
    if hstart + hint.len() <= max {
        for (i, c) in hint.chars().enumerate() {
            out.push(CellView {
                col: 3 + (hstart + i) as u16,
                row: 1,
                c,
                fg: DIM,
                bg: BG,
                bold: false,
                italic: false,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_color_thresholds() {
        // 4 cores: 1.0 load → 0.25/core (green); 3.0 → 0.75 (amber); 5.0 → 1.25 (red)
        assert_eq!(load_color(1.0, 4.0), accent());
        assert_eq!(load_color(3.0, 4.0), AMBER);
        assert_eq!(load_color(5.0, 4.0), RED);
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
