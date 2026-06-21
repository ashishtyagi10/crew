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

/// Render the CREW banner + tagline, centered in a `cols × rows` cell grid.
pub fn welcome_cells(cols: u16, rows: u16) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let mut cells = Vec::new();
    let bw = banner_width();
    let start_col = cols.saturating_sub(bw) / 2;
    let start_row = rows.saturating_sub(TOTAL_HEIGHT) / 2;

    for (i, line) in BANNER.lines().enumerate() {
        let row = start_row.saturating_add(i as u16);
        push_line(&mut cells, line, start_col, row, ACCENT, cols, rows);
    }

    // tagline: 6 banner lines + 1 gap = row offset 7
    let trow = start_row.saturating_add(7);
    let tw = TAGLINE.chars().count() as u16;
    let tcol = cols.saturating_sub(tw) / 2;
    push_line(&mut cells, TAGLINE, tcol, trow, DIM, cols, rows);

    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_size_non_empty_bounded_and_has_accent() {
        let cells = welcome_cells(80, 24);
        assert!(!cells.is_empty());
        assert!(cells.iter().all(|c| c.col < 80 && c.row < 24));
        assert!(cells.iter().any(|c| c.fg == (0, 255, 160)));
    }

    #[test]
    fn tiny_size_no_panic_and_in_bounds() {
        let cells = welcome_cells(2, 1);
        assert!(cells.iter().all(|c| c.col < 2 && c.row < 1));
    }
}
