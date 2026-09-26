//! What the dash's labels claim: the span its curve draws, the peak it
//! measured.
use super::super::{DashPane, SYS_TOP};

/// A week with nothing spent heads its bars `COST PER DAY` and stops there:
/// `peak $0.00` read as a meter showing zero. `/usage` says the same.
#[test]
fn an_empty_week_names_no_peak() {
    use crate::usageledger::{Buckets, DAYS, HOURS};
    let _g = crate::app::theme_test_guard();
    let mut b = Buckets {
        hourly: vec![0; DAYS * HOURS],
        daily_cost: vec![0; DAYS],
        tok_in: 0,
        tok_out: 0,
        cost_microusd: 0,
    };
    let text = |mut v: Vec<crew_render::CellView>| {
        v.sort_by_key(|c| (c.row, c.col));
        v.iter().map(|c| c.c).collect::<String>()
    };
    let mut d = DashPane::new();
    d.buckets = b.clone();
    let empty = text(d.cells(100, 40));
    assert!(
        empty.contains("COST PER DAY") && !empty.contains("peak"),
        "{empty}"
    );
    let usage = text(crate::usagepane::cells(&b, 100, 40));
    assert!(
        usage.contains("COST PER DAY") && !usage.contains("peak"),
        "{usage}"
    );
    b.daily_cost[3] = 900_000;
    d.buckets = b.clone();
    assert!(text(d.cells(100, 40)).contains("peak $0.90"));
    assert!(text(crate::usagepane::cells(&b, 100, 40)).contains("peak $0.90"));
}

/// The CPU label names the span the curve draws, not the history's capacity,
/// and says nothing over no curve at all.
#[test]
fn the_cpu_label_names_the_span_it_draws() {
    let _g = crate::app::theme_test_guard();
    let text = |d: &DashPane, cols| -> String {
        let mut v = d.cells(cols, 40);
        v.sort_by_key(|c| (c.row, c.col));
        v.iter().filter(|c| c.row == SYS_TOP).map(|c| c.c).collect()
    };
    let mut d = DashPane::new();
    d.cpu = crate::spark::History::new(240);
    assert!(!text(&d, 100).contains("CPU"), "no history, no label");
    for _ in 0..240 {
        d.cpu.push(10);
    }
    assert!(
        text(&d, 200).contains("CPU \u{b7} 4m00"),
        "{}",
        text(&d, 200)
    );
    let half = text(&d, 70);
    assert!(
        half.contains("CPU \u{b7} ") && !half.contains("4m00"),
        "{half}"
    );
}
