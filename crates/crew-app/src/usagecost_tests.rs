//! The cost chart when there is nothing to chart.
use super::{cells, layout};
use crate::usageledger::{Buckets, DAYS, HOURS};

fn week(daily_cost: Vec<u64>) -> Buckets {
    Buckets {
        hourly: vec![0; DAYS * HOURS],
        daily_cost,
        tok_in: 0,
        tok_out: 0,
        cost_microusd: 0,
    }
}

/// Each row's text, in column order.
fn rows_of(b: &Buckets, cols: u16, rows: u16) -> Vec<String> {
    let cells = cells(b, cols, rows);
    (0..rows)
        .map(|r| {
            let mut v: Vec<(u16, char)> = cells
                .iter()
                .filter(|c| c.row == r)
                .map(|c| (c.col, c.c))
                .collect();
            v.sort_unstable();
            v.into_iter().map(|(_, c)| c).collect()
        })
        .collect()
}

/// A week with nothing spent says so inside the plot, on one of its rows;
/// a week with spending draws its bars and no such note (glass survey F8).
#[test]
fn a_week_of_nothing_spent_says_so_in_the_plot() {
    let _g = crate::app::theme_test_guard();
    let rows = 40;
    let l = layout(rows);
    let note = "nothing spent in the last 7 days";
    let empty = rows_of(&week(vec![0; DAYS]), 80, rows);
    let row = empty
        .iter()
        .position(|t| t.contains(note))
        .expect("the note is drawn");
    let plot = usize::from(l.cost_top + 1)..usize::from(l.cost_top + 1 + l.cost_rows);
    assert!(plot.contains(&row), "{row} not in {plot:?}");
    let mut spent = vec![0; DAYS];
    spent[3] = 2_000_000;
    assert!(!rows_of(&week(spent), 80, rows)
        .iter()
        .any(|t| t.contains(note)));
}
