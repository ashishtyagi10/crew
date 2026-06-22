//! Pure rendering of the system-stats sidebar section: a header + spaced gauges.
use crew_render::CellView;

use crate::boxdraw;
use crate::stats::Stats;

const FILL: (u8, u8, u8) = (0, 255, 160);
const TRACK: (u8, u8, u8) = (70, 70, 80);
const BG: (u8, u8, u8) = (0, 0, 0);
const LABEL: (u8, u8, u8) = (200, 200, 200);
const BORDER: (u8, u8, u8) = (110, 110, 120);
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

/// Render the stats section as a rounded card with a `SYSTEM` legend embedded in
/// the top border (fieldset/legend style) and CPU/MEM/DISK gauges spaced on rows
/// 2, 4, 6. Future sidebar sections can stack as their own titled cards below.
pub(crate) fn render_stats(stats: Stats, cols: u16, rows: u16) -> Vec<CellView> {
    let mut out = Vec::new();
    if cols < 8 || rows < 6 {
        return out;
    }
    let left = 1u16;
    let right = cols - 2;
    let top = 0u16;
    let bottom = (top + 7).min(rows - 1);
    out.extend(boxdraw::titled_box(
        boxdraw::BoxRect {
            left,
            top,
            right,
            bottom,
        },
        HEADER,
        BORDER,
        FILL,
        BG,
    ));

    // Content indented one column inside the card border.
    let cstart = left + 2;
    let inner = right.saturating_sub(cstart + 1);

    let gauges = [
        ("CPU ", stats.cpu),
        ("MEM ", stats.mem),
        ("DISK", stats.disk),
    ];
    let mut row = top + 2;
    for (label, frac) in gauges {
        if row >= bottom {
            break;
        }
        for mut g in gauge_cells(label, frac, 0, inner) {
            g.col += cstart;
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
    fn render_stats_card_legend_and_gauges() {
        let stats = Stats {
            cpu: 0.1,
            mem: 0.2,
            disk: 0.3,
        };
        let cells = render_stats(stats, 24, 12);
        let has = |ch: char| cells.iter().any(|c| c.c == ch);
        // rounded card border
        assert!(has('╭') && has('╮') && has('╰') && has('╯'));
        // SYSTEM legend embedded on the top border row (row 0)
        assert!(cells.iter().any(|c| c.c == 'S' && c.row == 0));
        // gauge bars present, spaced on rows 2/4/6
        assert!(cells.iter().any(|c| c.c == '█' || c.c == '░'));
        let rows: std::collections::HashSet<u16> = cells.iter().map(|c| c.row).collect();
        assert!(rows.contains(&2) && rows.contains(&4) && rows.contains(&6));
    }
}
