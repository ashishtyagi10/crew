//! Sidebar clock card: a rounded `TIME` fieldset showing the local time and date.
use crew_render::CellView;

use crate::boxdraw::{titled_box, BoxRect};

const ACCENT: (u8, u8, u8) = (0, 255, 160);
const LABEL: (u8, u8, u8) = (200, 200, 200);
const BORDER: (u8, u8, u8) = (110, 110, 120);
const BG: (u8, u8, u8) = (0, 0, 0);

/// Rows the clock card occupies, including a one-row gap below it.
pub const CLOCK_H: u16 = 5;

/// Current local `(time, date)` as display strings, e.g. `("14:03:09", "Sat 21 Jun")`.
pub fn now_strings() -> (String, String) {
    let now = chrono::Local::now();
    (
        now.format("%H:%M:%S").to_string(),
        now.format("%a %d %b").to_string(),
    )
}

/// Render the clock card (rows 0..3) with `time` and `date` centered inside.
pub fn clock_cells(time: &str, date: &str, cols: u16) -> Vec<CellView> {
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
        "TIME",
        BORDER,
        ACCENT,
        BG,
    );
    put_centered(&mut out, time, 1, cols, ACCENT, true);
    put_centered(&mut out, date, 2, cols, LABEL, false);
    out
}

fn put_centered(
    out: &mut Vec<CellView>,
    s: &str,
    row: u16,
    cols: u16,
    fg: (u8, u8, u8),
    bold: bool,
) {
    let w = s.chars().count() as u16;
    let start = if w < cols { (cols - w) / 2 } else { 0 };
    for (i, c) in s.chars().enumerate() {
        let col = start + i as u16;
        if col >= cols {
            break;
        }
        out.push(CellView {
            col,
            row,
            c,
            fg,
            bg: BG,
            bold,
            italic: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_card_has_border_and_centered_time() {
        let cells = clock_cells("14:03:09", "Sat 21 Jun", 24);
        assert!(cells.iter().any(|c| c.c == '╭'));
        // TIME legend on the top border
        assert!(cells.iter().any(|c| c.c == 'T' && c.row == 0));
        // time digits on row 1
        assert!(cells.iter().any(|c| c.c == '1' && c.row == 1));
    }

    #[test]
    fn narrow_card_renders_nothing() {
        assert!(clock_cells("12:00:00", "Mon", 6).is_empty());
    }
}
