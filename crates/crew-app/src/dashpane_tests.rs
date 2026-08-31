use super::{
    layout, DashPane, COST_MAX, COST_MIN, HEAT_ROW_MAX, MIN_COLS, NET_TOP, SYS_TOP, USE_TOP,
};

/// The bands are drawn in priority order, so a pane that cannot hold the
/// history still holds the machine. A dashboard that vanishes below some
/// height is worse than one that says less.
#[test]
fn a_short_pane_keeps_the_machine_and_loses_the_history() {
    let _g = crate::app::theme_test_guard();
    let d = DashPane::new();
    let rows_of = |rows: u16| -> Vec<u16> {
        let mut v: Vec<u16> = d
            .cells(100, rows)
            .into_iter()
            .map(|c| c.row)
            .chain(
                d.paint(100, rows, 2.0)
                    .into_iter()
                    .map(|p| p.y.floor() as u16),
            )
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let tall = rows_of(40);
    assert!(
        tall.iter().any(|&r| r >= layout(40).cost_top),
        "the tall pane has every band"
    );
    let short = rows_of(NET_TOP + 1);
    assert!(!short.is_empty(), "the machine is still drawn");
    assert!(
        short.iter().all(|&r| r < USE_TOP),
        "a short pane drew a band it has no room for: {short:?}"
    );
}

/// A tall dashboard spends its height on the two bands that get truer with
/// rows, instead of finishing 55% of the way down.
#[test]
fn a_tall_pane_gives_its_slack_to_the_histories() {
    let short = layout(30);
    let tall = layout(56);
    assert!(
        tall.heat_h > short.heat_h,
        "the heatmap grew: {} vs {}",
        tall.heat_h,
        short.heat_h
    );
    assert!(
        tall.cost_rows > short.cost_rows,
        "and so did the cost curve: {} vs {}",
        tall.cost_rows,
        short.cost_rows
    );
    let huge = layout(300);
    assert_eq!(huge.heat_h, HEAT_ROW_MAX, "but not without a cap");
    assert_eq!(huge.cost_rows, COST_MAX);
}

/// At every height a tile can be: the cost curve stays inside the pane,
/// and it is given up rather than drawn as a one-row smear.
#[test]
fn the_cost_curve_never_runs_past_the_last_row() {
    for rows in 0..140u16 {
        let l = layout(rows);
        assert!((1..=HEAT_ROW_MAX).contains(&l.heat_h), "{rows}: {l:?}");
        if l.cost_rows > 0 {
            assert!(l.cost_rows >= COST_MIN, "{rows}: {l:?}");
            assert!(
                l.cost_top + l.cost_rows < rows,
                "{rows}: the curve is off the pane: {l:?}"
            );
            // And its legend sits on a row the heatmap above it has let go.
            assert!(
                l.cost_top > USE_TOP + crate::usageledger::DAYS as u16 * l.heat_h,
                "{rows}: the legend is on the heatmap: {l:?}"
            );
        }
    }
}

#[test]
fn a_narrow_pane_draws_nothing_rather_than_a_mess() {
    let _g = crate::app::theme_test_guard();
    let d = DashPane::new();
    assert!(d.cells(MIN_COLS - 1, 40).is_empty());
    assert!(d.paint(MIN_COLS - 1, 40, 2.0).is_empty());
}

#[test]
fn every_band_stays_in_its_own_rows() {
    let _g = crate::app::theme_test_guard();
    let d = DashPane::new();
    // The dials own the SYSTEM band; nothing they draw may reach the NET
    // header a row below it — at any pane width, since the block gives
    // width back to the curve rather than squeezing it out.
    let dials: Vec<_> = [MIN_COLS, 60, 120]
        .into_iter()
        .flat_map(|cols| {
            crate::sysdials::DASH.paint(d.sampler.stats(), super::ring_w(cols), SYS_TOP, 2.0)
        })
        .collect();
    for p in dials {
        assert!(p.y >= f32::from(SYS_TOP), "{p:?}");
        assert!(
            p.y + p.h <= f32::from(NET_TOP - 1) + 1e-3,
            "a dial reached the NET band: {p:?}"
        );
    }
}

#[test]
fn the_dashboard_draws_something_on_a_real_pane() {
    let _g = crate::app::theme_test_guard();
    let mut d = DashPane::new();
    for v in 0..40u64 {
        d.cpu.push(v * 2 % 100);
    }
    assert!(!d.cells(110, 36).is_empty());
    assert!(!d.paint(110, 36, 2.0).is_empty(), "the widgets drew");
}
