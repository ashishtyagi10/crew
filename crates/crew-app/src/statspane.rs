use crew_render::CellView;

use crate::stats::{Stats, SysSampler};

const FILL: (u8, u8, u8) = (0, 255, 160);
const TRACK: (u8, u8, u8) = (40, 80, 95);
const BG: (u8, u8, u8) = (8, 8, 16);
const LABEL: (u8, u8, u8) = (200, 200, 200);

pub struct StatsPane {
    sampler: SysSampler,
}

impl StatsPane {
    pub fn new() -> Self {
        Self {
            sampler: SysSampler::new(),
        }
    }

    pub fn refresh(&mut self) -> bool {
        self.sampler.refresh()
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render_stats(self.sampler.stats(), cols, rows)
    }
}

impl Default for StatsPane {
    fn default() -> Self {
        Self::new()
    }
}

/// Emit a single row of cells for a gauge bar: label | space | bar | pct.
pub fn gauge_cells(label: &str, frac: f32, row: u16, cols: u16) -> Vec<CellView> {
    if cols == 0 {
        return Vec::new();
    }
    let cols = cols as usize;
    let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u16;
    let pct_str = format!("{pct:>3}%");
    let pct_len = pct_str.len(); // always 4

    let label_chars: Vec<char> = label.chars().collect();
    let label_len = label_chars.len();

    // Build the full cell list in order, then truncate to `cols`.
    let mut cells: Vec<CellView> = Vec::with_capacity(cols);

    // 1. Label
    for (i, &c) in label_chars.iter().enumerate() {
        if cells.len() >= cols {
            break;
        }
        cells.push(CellView {
            col: i as u16,
            row,
            c,
            fg: LABEL,
            bg: BG,
            bold: false,
            italic: false,
        });
    }

    // 2. Space after label
    if cells.len() < cols {
        cells.push(CellView {
            col: (label_len) as u16,
            row,
            c: ' ',
            fg: LABEL,
            bg: BG,
            bold: false,
            italic: false,
        });
    }

    // 3. Bar (if room for at least one bar cell and the pct text)
    let used_so_far = cells.len();
    let bar_width = cols.saturating_sub(label_len + 1 + pct_len);
    let filled = (frac.clamp(0.0, 1.0) * bar_width as f32).round() as usize;

    for i in 0..bar_width {
        if cells.len() >= cols {
            break;
        }
        let (c, fg) = if i < filled {
            ('█', FILL)
        } else {
            ('░', TRACK)
        };
        cells.push(CellView {
            col: (used_so_far + i) as u16,
            row,
            c,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        });
    }

    // 4. Percent text
    let pct_start = cols.saturating_sub(pct_len);
    for (i, c) in pct_str.chars().enumerate() {
        let col = pct_start + i;
        if col >= cols {
            break;
        }
        // Overwrite or append
        if col < cells.len() {
            cells[col] = CellView {
                col: col as u16,
                row,
                c,
                fg: LABEL,
                bg: BG,
                bold: false,
                italic: false,
            };
        } else {
            cells.push(CellView {
                col: col as u16,
                row,
                c,
                fg: LABEL,
                bg: BG,
                bold: false,
                italic: false,
            });
        }
    }

    cells
}

/// Render CPU, MEM, DISK gauges as rows 0, 1, 2 (clipped to `rows`).
pub fn render_stats(stats: Stats, cols: u16, rows: u16) -> Vec<CellView> {
    let gauges = [
        ("CPU ", stats.cpu),
        ("MEM ", stats.mem),
        ("DISK", stats.disk),
    ];
    let mut out = Vec::new();
    for (row, (label, frac)) in gauges.iter().enumerate() {
        if row as u16 >= rows {
            break;
        }
        out.extend(gauge_cells(label, *frac, row as u16, cols));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauge_50_pct_all_cells_on_row_0() {
        let cells = gauge_cells("CPU ", 0.5, 0, 40);
        assert!(!cells.is_empty(), "should emit cells");
        for c in &cells {
            assert_eq!(c.row, 0, "every cell must be on row 0");
        }
        let filled = cells.iter().filter(|c| c.c == '█').count();
        let track = cells.iter().filter(|c| c.c == '░').count();
        let diff = (filled as i32 - track as i32).unsigned_abs() as usize;
        assert!(diff <= 1, "filled={filled} track={track} diff must be ≤1");
    }

    #[test]
    fn gauge_0_pct_no_filled() {
        let cells = gauge_cells("CPU ", 0.0, 0, 40);
        let filled = cells.iter().filter(|c| c.c == '█').count();
        assert_eq!(filled, 0, "0% must have no filled cells");
    }

    #[test]
    fn gauge_100_pct_no_track() {
        let cells = gauge_cells("CPU ", 1.0, 0, 40);
        let track = cells.iter().filter(|c| c.c == '░').count();
        assert_eq!(track, 0, "100% must have no track cells");
    }

    #[test]
    fn render_stats_three_rows() {
        let stats = Stats {
            cpu: 0.1,
            mem: 0.2,
            disk: 0.3,
        };
        let cells = render_stats(stats, 40, 3);
        let rows: std::collections::HashSet<u16> = cells.iter().map(|c| c.row).collect();
        assert_eq!(rows, [0u16, 1, 2].into_iter().collect());
    }
}
