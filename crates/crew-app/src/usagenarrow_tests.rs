//! `/usage` at its narrowest widths: nothing overprints, and the ring's
//! key stays a key. `COST PER DAY` filled columns 1–12 while `peak $…` was
//! put at `cols - 14`, so at 24–27 columns two cells landed on one square
//! and the last writer won; the `in   184k  55%` legend at column 13 needed
//! 27 columns and was cut mid-number instead.
use crew_render::CellView;

fn buckets() -> crate::usageledger::Buckets {
    use crate::usageledger::{DAYS, HOURS};
    crate::usageledger::Buckets {
        hourly: vec![0u64; DAYS * HOURS],
        daily_cost: vec![120_000, 340_000, 0, 20_000, 810_000, 430_000, 260_000],
        tok_in: 1_840_000,
        tok_out: 410_000,
        cost_microusd: 1_980_000,
    }
}

fn text(cells: &[CellView], row: u16) -> String {
    let mut on: Vec<&CellView> = cells.iter().filter(|c| c.row == row).collect();
    on.sort_by_key(|c| c.col);
    on.iter().map(|c| c.c).collect()
}

fn all(cells: &[CellView]) -> String {
    (0..40)
        .map(|r| text(cells, r))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn no_two_cells_share_a_square_at_any_narrow_width() {
    let _g = crate::app::theme_test_guard();
    for cols in 24..=40u16 {
        let cells = super::cells(&buckets(), cols, 30);
        let mut seen = std::collections::HashSet::new();
        for c in &cells {
            assert!(
                seen.insert((c.row, c.col)),
                "{cols} cols: two cells at {:?}",
                (c.row, c.col)
            );
        }
    }
}

#[test]
fn the_narrow_pane_keeps_the_key_and_drops_the_peak() {
    let _g = crate::app::theme_test_guard();
    let narrow = all(&super::cells(&buckets(), 24, 30));
    assert!(narrow.contains("in 8"), "{narrow}");
    assert!(narrow.contains("out 1"), "{narrow}");
    for key in narrow
        .lines()
        .filter(|l| l.starts_with("in ") || l.starts_with("out "))
    {
        assert!(!key.contains('\u{2026}'), "{narrow}");
    }
    assert!(!narrow.contains("peak"), "{narrow}");
    let wide = all(&super::cells(&buckets(), 40, 30));
    assert!(wide.contains("in   1.8M  82%"), "{wide}");
    assert!(wide.contains("peak $0.81"), "{wide}");
}

/// The cost bars stand under the heat grid's own span, so the two charts
/// share their edges: they ran from column 1 to a column short of the
/// grid's end — 24px from the left stroke, 46px from the right (glass
/// survey D-L8).
#[test]
fn the_cost_bars_span_the_heat_grid_above_them() {
    let _g = crate::app::theme_test_guard();
    let b = crate::usageledger::Buckets {
        hourly: vec![5; crate::usageledger::DAYS * crate::usageledger::HOURS],
        daily_cost: vec![1, 4, 2, 9, 3, 6, 5],
        tok_in: 10,
        tok_out: 5,
        cost_microusd: 100,
    };
    let (cols, rows) = (60u16, 34u16);
    let l = super::layout(rows);
    let p = super::paint(&b, cols, rows, 2.0);
    let span = |top: f32, end: f32| {
        let band = p.iter().filter(|q| q.y >= top && q.y < end);
        let lo = band.clone().map(|q| q.x).fold(f32::MAX, f32::min);
        (lo, band.map(|q| q.x + q.w).fold(0.0, f32::max))
    };
    let heat = span(f32::from(super::HEAT_TOP), f32::from(l.split_top));
    let cost = span(f32::from(l.cost_top), f32::from(rows));
    let day = (heat.1 - heat.0) / crate::usageledger::DAYS as f32;
    assert!((heat.0 - f32::from(super::LABEL_W)).abs() < 0.2, "{heat:?}");
    // The outer bars keep their slots' own air, the same at both ends.
    let (l_air, r_air) = (cost.0 - heat.0, heat.1 - cost.1);
    assert!((l_air - r_air).abs() < 0.1, "{cost:?} vs {heat:?}");
    assert!((0.0..day / 4.0).contains(&l_air), "{cost:?} vs {heat:?}");
}
