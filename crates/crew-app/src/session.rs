use crew_term::{GridSize, RenderCell};

/// Flatten visible cells into rows of text for the single-string renderer.
pub fn cells_to_string(cells: &[RenderCell], size: GridSize) -> String {
    let mut grid = vec![vec![' '; size.cols as usize]; size.rows as usize];
    for c in cells {
        if (c.row as usize) < grid.len() && (c.col as usize) < grid[0].len() {
            grid[c.row as usize][c.col as usize] = c.c;
        }
    }
    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
