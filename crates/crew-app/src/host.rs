//! Sidebar host card: a rounded `HOST` fieldset showing the machine name, OS,
//! and uptime — static system info that complements the live clock + gauges.
use crew_render::CellView;

use crate::boxdraw::{titled_box, BoxRect};

const ACCENT: (u8, u8, u8) = (0, 255, 160);
const LABEL: (u8, u8, u8) = (200, 200, 200);
const DIM: (u8, u8, u8) = (150, 150, 160);
const BORDER: (u8, u8, u8) = (110, 110, 120);
const BG: (u8, u8, u8) = (0, 0, 0);

/// Current `(name, uptime)` display strings, e.g. `("mbp · macOS", "up 3h 12m")`.
pub fn host_strings() -> (String, String) {
    let host = sysinfo::System::host_name().unwrap_or_else(|| "crew".to_string());
    let os = sysinfo::System::name().unwrap_or_default();
    let name = if os.is_empty() {
        host
    } else {
        format!("{host} · {os}")
    };
    (name, fmt_uptime(sysinfo::System::uptime()))
}

/// Format seconds of uptime compactly: `up 2d 3h`, `up 3h 12m`, or `up 12m`.
fn fmt_uptime(secs: u64) -> String {
    let (d, h, m) = (secs / 86400, (secs % 86400) / 3600, (secs % 3600) / 60);
    if d > 0 {
        format!("up {d}d {h}h")
    } else if h > 0 {
        format!("up {h}h {m}m")
    } else {
        format!("up {m}m")
    }
}

/// Render the host card (rows 0..3) with `name` and `uptime` inside.
pub fn host_cells(name: &str, uptime: &str, cols: u16) -> Vec<CellView> {
    if cols < 10 {
        return Vec::new();
    }
    let mut out = titled_box(
        BoxRect {
            left: 1,
            top: 0,
            right: cols - 2,
            bottom: 3,
        },
        "HOST",
        BORDER,
        ACCENT,
        BG,
    );
    put(&mut out, name, 1, cols, LABEL);
    put(&mut out, uptime, 2, cols, DIM);
    out
}

/// Draw `s` at `row`, indented one column inside the card border, clipped to `cols`.
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
    fn fmt_uptime_buckets() {
        assert_eq!(fmt_uptime(30), "up 0m");
        assert_eq!(fmt_uptime(12 * 60), "up 12m");
        assert_eq!(fmt_uptime(3 * 3600 + 12 * 60), "up 3h 12m");
        assert_eq!(fmt_uptime(2 * 86400 + 3 * 3600), "up 2d 3h");
    }

    #[test]
    fn host_card_has_border_and_name() {
        let cells = host_cells("mbp · macOS", "up 1h 2m", 24);
        assert!(cells.iter().any(|c| c.c == '╭'));
        assert!(cells.iter().any(|c| c.c == 'H' && c.row == 0)); // HOST legend
        assert!(cells.iter().any(|c| c.c == 'm' && c.row == 1)); // name
    }
}
