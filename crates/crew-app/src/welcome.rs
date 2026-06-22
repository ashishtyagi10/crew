//! The empty-screen welcome: matrix digital rain with the word CREW woven into
//! the center — the letters sit bright and persistent while rain flows through
//! the gaps between them, so the wordmark feels part of the rain.
use crew_render::CellView;

/// Bright white-green for the wordmark (matches the rain's head colour).
const HEAD: (u8, u8, u8) = (210, 255, 220);
const BG: (u8, u8, u8) = (0, 0, 0);
const WORD: &str = "CREW";
/// Columns between successive letters (one rain cell shows through each gap).
const STEP: u16 = 2;

/// Render one animation frame: rain everywhere, CREW overlaid in the center.
pub fn welcome_cells_animated(cols: u16, rows: u16, tick: u64) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let mut cells = crate::matrix::rain(cols, rows, tick);

    let letters: Vec<char> = WORD.chars().collect();
    let span = (letters.len() as u16 - 1) * STEP + 1;
    if span >= cols {
        return cells; // too narrow for the wordmark; just rain
    }
    let start_col = (cols - span) / 2;
    let row = rows / 2;
    // Overlaid last so the letters win over any rain glyph in their cell, while
    // the cells between them keep showing the rain underneath.
    for (i, &ch) in letters.iter().enumerate() {
        cells.push(CellView {
            col: start_col + i as u16 * STEP,
            row,
            c: ch,
            fg: HEAD,
            bg: BG,
            bold: true,
            italic: false,
        });
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weaves_crew_into_rain_in_bounds() {
        let cells = welcome_cells_animated(80, 24, 7);
        assert!(cells.iter().all(|c| c.col < 80 && c.row < 24));
        // every CREW letter is present, bright and on the center row
        for ch in WORD.chars() {
            assert!(cells
                .iter()
                .any(|c| c.c == ch && c.fg == HEAD && c.row == 12));
        }
    }

    #[test]
    fn tiny_size_no_panic_and_in_bounds() {
        let cells = welcome_cells_animated(2, 1, 0);
        assert!(cells.iter().all(|c| c.col < 2 && c.row < 1));
    }
}
