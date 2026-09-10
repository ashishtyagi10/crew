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
    assert!(wide.contains("in   1.8M  81%"), "{wide}");
    assert!(wide.contains("peak $0.81"), "{wide}");
}
