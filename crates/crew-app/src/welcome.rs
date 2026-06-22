//! The empty-screen welcome: matrix digital rain with the word CREW woven into
//! the center — the letters sit bright and persistent while rain flows through
//! the gaps between them, so the wordmark feels part of the rain.
use crew_render::CellView;

/// Bright white-green for a letter at the peak of its pulse (= the rain's head).
const HEAD: (u8, u8, u8) = (210, 255, 220);
const BG: (u8, u8, u8) = (0, 0, 0);
const WORD: &str = "CREW";
/// Dim grey for the discoverability hint under the wordmark.
const HINT_FG: (u8, u8, u8) = (110, 110, 120);
const HINT: &str = "Cmd+T  new shell    ·    /  commands";
/// Faint colour for the version stamp in the corner.
const VERSION_FG: (u8, u8, u8) = (70, 75, 85);
/// Columns between successive letters (one rain cell shows through each gap).
const STEP: u16 = 2;
/// Frames for one brighten→dim→brighten pulse of a letter.
const PULSE: u64 = 56;

/// Rain-like colour for letter `i` at `tick`: each letter pulses between the
/// bright head and a dim green (never fully gone), out of phase with the others.
fn letter_style(tick: u64, i: usize) -> ((u8, u8, u8), bool) {
    let phase = (tick / 2 + i as u64 * 11) % PULSE;
    let half = PULSE / 2;
    let dist = if phase < half { phase } else { PULSE - phase }; // 0 = brightest
    if dist == 0 {
        return (HEAD, true);
    }
    let frac = dist as f32 / half as f32; // 0..1 → bright..dim
    let g = (235.0 - frac * 150.0) as u8; // 235..85
    ((0, g, g / 6), false)
}

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
        let (fg, bold) = letter_style(tick, i);
        cells.push(CellView {
            col: start_col + i as u16 * STEP,
            row,
            c: ch,
            fg,
            bg: BG,
            bold,
            italic: false,
        });
    }

    // A dim hint two rows below the wordmark, when it fits.
    let hint_w = HINT.chars().count() as u16;
    let hint_row = row + 2;
    if hint_w < cols && hint_row < rows {
        let hstart = (cols - hint_w) / 2;
        for (i, ch) in HINT.chars().enumerate() {
            cells.push(CellView {
                col: hstart + i as u16,
                row: hint_row,
                c: ch,
                fg: HINT_FG,
                bg: BG,
                bold: false,
                italic: false,
            });
        }
    }

    // Version stamp in the bottom-right corner.
    let ver = concat!("v", env!("CARGO_PKG_VERSION"));
    let vw = ver.chars().count() as u16;
    if vw + 1 < cols && rows > 0 {
        let vstart = cols - vw - 1;
        for (i, ch) in ver.chars().enumerate() {
            cells.push(CellView {
                col: vstart + i as u16,
                row: rows - 1,
                c: ch,
                fg: VERSION_FG,
                bg: BG,
                bold: false,
                italic: false,
            });
        }
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
        // every CREW letter is present on the center row in a green-dominant hue
        for ch in WORD.chars() {
            assert!(cells
                .iter()
                .any(|c| c.c == ch && c.row == 12 && c.fg.1 >= c.fg.0 && c.fg.1 > 80));
        }
        // the dim hint is shown two rows below the wordmark
        assert!(cells.iter().any(|c| c.row == 14 && c.fg == HINT_FG));
        // a faint version stamp sits on the bottom row
        assert!(cells
            .iter()
            .any(|c| c.c == 'v' && c.row == 23 && c.fg == VERSION_FG));
    }

    #[test]
    fn letters_pulse_over_time() {
        // a letter's colour changes between frames (shimmer, not constant)
        let a = letter_style(0, 0);
        let b = letter_style(20, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn tiny_size_no_panic_and_in_bounds() {
        let cells = welcome_cells_animated(2, 1, 0);
        assert!(cells.iter().all(|c| c.col < 2 && c.row < 1));
    }
}
