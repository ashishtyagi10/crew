use crew_render::CellView;

const ACCENT: (u8, u8, u8) = (0, 255, 160);
const DIM: (u8, u8, u8) = (120, 120, 140);
const BG: (u8, u8, u8) = (8, 8, 16);
const TAGLINE: &str = "the next-gen terminal";
const TOTAL_HEIGHT: u16 = 8; // 6 banner + 1 gap + 1 tagline

const BANNER: &str = " ██████╗██████╗ ███████╗██╗    ██╗\n\
██╔════╝██╔══██╗██╔════╝██║    ██║\n\
██║     ██████╔╝█████╗  ██║ █╗ ██║\n\
██║     ██╔══██╗██╔══╝  ██║███╗██║\n\
╚██████╗██║  ██║███████╗╚███╔███╔╝\n\
 ╚═════╝╚═╝  ╚═╝╚══════╝ ╚══╝╚══╝";

fn banner_width() -> u16 {
    BANNER.lines().map(|l| l.chars().count()).max().unwrap_or(0) as u16
}

fn push_line(
    cells: &mut Vec<CellView>,
    line: &str,
    start_col: u16,
    row: u16,
    fg: (u8, u8, u8),
    cols: u16,
    rows: u16,
) {
    if row >= rows {
        return;
    }
    for (i, c) in line.chars().enumerate() {
        let col = start_col.saturating_add(i as u16);
        if col >= cols {
            break;
        }
        cells.push(CellView {
            col,
            row,
            c,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        });
    }
}

/// Blank a rectangular region to the background colour so the logo reads crisply
/// over the rain. Coordinates are clamped to the grid.
fn blank_box(cells: &mut Vec<CellView>, c0: u16, r0: u16, c1: u16, r1: u16, cols: u16, rows: u16) {
    for row in r0..=r1.min(rows.saturating_sub(1)) {
        for col in c0..=c1.min(cols.saturating_sub(1)) {
            cells.push(CellView {
                col,
                row,
                c: ' ',
                fg: BG,
                bg: BG,
                bold: false,
                italic: false,
            });
        }
    }
}

/// Render the matrix-rain welcome: digital rain behind a crisp, centered CREW
/// banner + tagline. `tick` advances the animation.
pub fn welcome_cells_animated(cols: u16, rows: u16, tick: u64) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let mut cells = crate::matrix::rain(cols, rows, tick);
    let bw = banner_width();
    let start_col = cols.saturating_sub(bw) / 2;
    let start_row = rows.saturating_sub(TOTAL_HEIGHT) / 2;
    // Clear a padded box behind the logo + tagline, then draw them on top.
    blank_box(
        &mut cells,
        start_col.saturating_sub(2),
        start_row.saturating_sub(1),
        start_col + bw + 1,
        start_row + TOTAL_HEIGHT,
        cols,
        rows,
    );
    draw_logo(&mut cells, start_col, start_row, cols, rows);
    cells
}

/// Draw the CREW banner (accent) and tagline (dim) at the given origin.
fn draw_logo(cells: &mut Vec<CellView>, start_col: u16, start_row: u16, cols: u16, rows: u16) {
    for (i, line) in BANNER.lines().enumerate() {
        let row = start_row.saturating_add(i as u16);
        push_line(cells, line, start_col, row, ACCENT, cols, rows);
    }
    let trow = start_row.saturating_add(7);
    let tw = TAGLINE.chars().count() as u16;
    let tcol = cols.saturating_sub(tw) / 2;
    push_line(cells, TAGLINE, tcol, trow, DIM, cols, rows);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_size_bounded_with_logo_and_rain() {
        let cells = welcome_cells_animated(80, 24, 7);
        assert!(!cells.is_empty());
        assert!(cells.iter().all(|c| c.col < 80 && c.row < 24));
        // the accent CREW banner is drawn on top of the rain
        assert!(cells.iter().any(|c| c.fg == ACCENT));
    }

    #[test]
    fn tiny_size_no_panic_and_in_bounds() {
        let cells = welcome_cells_animated(2, 1, 0);
        assert!(cells.iter().all(|c| c.col < 2 && c.row < 1));
    }
}
