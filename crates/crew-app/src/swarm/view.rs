//! Fleet → CellViews renderer: maps a `crew_hive::Fleet` to a flat list of
//! `crew_render::CellView`s suitable for the GPU pane, plus a HUD row 0.
use crew_hive::view::{fleet_view, render_cells, Rgb};
use crew_hive::{Fleet, TaskGraph};
use crew_render::CellView;

/// Map a `Fleet` to a `Vec<CellView>` for the given terminal grid.
///
/// Row 0 is a HUD showing live/done/failed/cost totals. Constellation or
/// heatmap glyphs occupy rows 1‥rows-1 (shifted down by 1).
///
/// Returns an empty vec when `cols == 0 || rows == 0`.
pub fn swarm_cells(graph: &TaskGraph, fleet: &Fleet, cols: u16, rows: u16) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return vec![];
    }

    // Reserve row 0 for the HUD; content gets the remaining rows.
    let content_rows = rows.saturating_sub(1);
    let view = fleet_view(graph, fleet, cols as usize);
    let glyphs = render_cells(&view, cols, content_rows);

    let mut cells: Vec<CellView> = glyphs
        .into_iter()
        .map(|g| {
            let Rgb(r, gv, b) = g.color;
            CellView {
                col: g.col,
                row: g.row.saturating_add(1), // shift below HUD
                c: g.ch,
                fg: (r, gv, b),
                bg: (0, 0, 0),
                bold: false,
                italic: false,
            }
        })
        .collect();

    // HUD row: live/done/failed + cost in dollars.
    let t = fleet.totals();
    let hud = format!(
        " live:{} done:{} failed:{} cost:${:.4}",
        t.live,
        t.done,
        t.failed,
        t.micros_usd as f64 / 1_000_000.0,
    );
    for (col, ch) in hud.chars().enumerate() {
        if col as u16 >= cols {
            break;
        }
        cells.push(CellView {
            col: col as u16,
            row: 0,
            c: ch,
            fg: (200, 200, 200),
            bg: (20, 20, 40),
            bold: false,
            italic: false,
        });
    }

    cells
}
