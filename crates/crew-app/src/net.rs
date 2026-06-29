//! Sidebar network section: a `NET` divider above down/up byte rates.
use crew_render::CellView;

use crate::boxdraw::section_header;

use crate::palette::accent;
const LABEL: (u8, u8, u8) = (200, 200, 200);
const DIM: (u8, u8, u8) = (150, 150, 160);
const BORDER: (u8, u8, u8) = (110, 110, 120);
const BG: (u8, u8, u8) = (0, 0, 0);

/// Format a per-second byte rate compactly, e.g. `0 B/s`, `12 KB/s`, `3.4 MB/s`.
pub fn rate(bytes: u64) -> String {
    let b = bytes as f64;
    if b < 1024.0 {
        format!("{bytes} B/s")
    } else if b < 1024.0 * 1024.0 {
        format!("{:.0} KB/s", b / 1024.0)
    } else {
        format!("{:.1} MB/s", b / (1024.0 * 1024.0))
    }
}

/// Render the network section: a `NET` rule on row 0, `↓ rx` on row 1, `↑ tx`
/// on row 2.
pub fn net_cells(rx: u64, tx: u64, cols: u16) -> Vec<CellView> {
    if cols < 10 {
        return Vec::new();
    }
    let mut out = section_header("NET", cols, BORDER, accent(), BG);
    put(&mut out, &format!("↓ {}", rate(rx)), 1, cols, LABEL);
    put(&mut out, &format!("↑ {}", rate(tx)), 2, cols, DIM);
    out
}

fn put(out: &mut Vec<CellView>, s: &str, row: u16, cols: u16, fg: (u8, u8, u8)) {
    let max = cols.saturating_sub(4) as usize;
    for (i, c) in s.chars().take(max).enumerate() {
        out.push(CellView {
            col: 3 + i as u16,
            row,
            c,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_units() {
        assert_eq!(rate(0), "0 B/s");
        assert_eq!(rate(500), "500 B/s");
        assert_eq!(rate(2048), "2 KB/s");
        assert_eq!(rate(3_500_000), "3.3 MB/s");
    }

    #[test]
    fn net_section_has_rule_and_arrows() {
        let cells = net_cells(2048, 1024, 24);
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        assert!(!cells.iter().any(|c| c.c == '╭'));
        assert!(cells.iter().any(|c| c.c == '↓' && c.row == 1));
        assert!(cells.iter().any(|c| c.c == '↑' && c.row == 2));
    }
}
