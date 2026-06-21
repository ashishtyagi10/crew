//! Pure rendering of the system-stats sidebar section: a header + spaced gauges.
use crew_render::CellView;

use crate::stats::Stats;

const FILL: (u8, u8, u8) = (0, 255, 160);
const TRACK: (u8, u8, u8) = (40, 80, 95);
const BG: (u8, u8, u8) = (8, 8, 16);
const LABEL: (u8, u8, u8) = (200, 200, 200);

/// Left/right padding (cols) and the header text for the section.
const PAD_L: u16 = 2;
const PAD_R: u16 = 2;
const HEADER: &str = "SYSTEM";

/// One gauge row laid out within `cols`: `label | space | bar | NNN%`.
fn gauge_cells(label: &str, frac: f32, row: u16, cols: u16) -> Vec<CellView> {
    if cols == 0 {
        return Vec::new();
    }
    let cols = cols as usize;
    let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u16;
    let pct_str = format!("{pct:>3}%");
    let pct_len = pct_str.len();

    let label_chars: Vec<char> = label.chars().collect();
    let label_len = label_chars.len();
    let mut cells: Vec<CellView> = Vec::with_capacity(cols);

    for (i, &c) in label_chars.iter().enumerate() {
        if cells.len() >= cols {
            break;
        }
        cells.push(cell(i as u16, row, c, LABEL));
    }
    if cells.len() < cols {
        cells.push(cell(label_len as u16, row, ' ', LABEL));
    }

    let used = cells.len();
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
        cells.push(cell((used + i) as u16, row, c, fg));
    }

    let pct_start = cols.saturating_sub(pct_len);
    for (i, c) in pct_str.chars().enumerate() {
        let col = pct_start + i;
        if col >= cols {
            break;
        }
        if col < cells.len() {
            cells[col] = cell(col as u16, row, c, LABEL);
        } else {
            cells.push(cell(col as u16, row, c, LABEL));
        }
    }
    cells
}

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg: BG,
        bold: false,
        italic: false,
    }
}

/// Render the stats section: a `SYSTEM` header (row 1) then CPU/MEM/DISK gauges
/// on rows 3, 5, 7 — left/right padded and vertically spaced for breathing room.
pub(crate) fn render_stats(stats: Stats, cols: u16, rows: u16) -> Vec<CellView> {
    let mut out = Vec::new();
    if cols <= PAD_L + PAD_R || rows == 0 {
        return out;
    }
    let inner = cols - PAD_L - PAD_R;

    if rows > 1 {
        for (i, c) in HEADER.chars().enumerate() {
            let col = PAD_L + i as u16;
            if col >= cols {
                break;
            }
            out.push(CellView {
                col,
                row: 1,
                c,
                fg: FILL,
                bg: BG,
                bold: true,
                italic: false,
            });
        }
    }

    let gauges = [
        ("CPU ", stats.cpu),
        ("MEM ", stats.mem),
        ("DISK", stats.disk),
    ];
    let mut row = 3u16;
    for (label, frac) in gauges {
        if row >= rows {
            break;
        }
        for mut g in gauge_cells(label, frac, 0, inner) {
            g.col += PAD_L;
            g.row = row;
            out.push(g);
        }
        row += 2;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn render_stats_spaced_and_padded() {
        let stats = Stats {
            cpu: 0.1,
            mem: 0.2,
            disk: 0.3,
        };
        let cells = render_stats(stats, 40, 12);
        let rows: std::collections::HashSet<u16> = cells.iter().map(|c| c.row).collect();
        // header on 1, gauges spaced on 3/5/7
        assert!(rows.contains(&1) && rows.contains(&3) && rows.contains(&5) && rows.contains(&7));
        // blank spacer rows
        assert!(!rows.contains(&0) && !rows.contains(&2) && !rows.contains(&4));
        // left padding
        assert!(cells.iter().all(|c| c.col >= PAD_L));
    }
}
