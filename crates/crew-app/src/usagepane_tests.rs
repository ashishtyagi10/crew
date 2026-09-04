use super::{
    cells, compact, layout, money, paint, COST_MAX, COST_MIN, HEAT_ROW_MAX, HEAT_TOP, LABEL_W,
    RING_ROW, RING_R_IN, SPLIT_ROWS,
};
use crate::usageledger::{Buckets, DAYS, HOURS};

/// A week with exactly one busy hour, in a named day/hour.
fn one_hot(day: usize, hour: usize) -> Buckets {
    let mut hourly = vec![0u64; DAYS * HOURS];
    hourly[day * HOURS + hour] = 10_000;
    Buckets {
        hourly,
        daily_cost: vec![0; DAYS],
        tok_in: 7_000,
        tok_out: 3_000,
        cost_microusd: 1_500_000,
    }
}

#[test]
fn the_heatmaps_hot_cell_lands_on_its_own_day_and_hour() {
    let _g = crate::app::theme_test_guard();
    // At every band height the pane's division can hand the grid: the
    // cell's row is what changes, the day it belongs to is not.
    for rows in [22u16, 30, 40] {
        let h = layout(rows).heat_h;
        for (day, hour) in [(0usize, 0usize), (3, 12), (DAYS - 1, HOURS - 1)] {
            let p = paint(&one_hot(day, hour), 60, rows, 2.0);
            let end = f32::from(HEAT_TOP + DAYS as u16 * h);
            let hot = p
                .iter()
                .filter(|r| r.y >= f32::from(HEAT_TOP) && r.y < end)
                .max_by(|a, b| a.alpha.total_cmp(&b.alpha))
                .expect("the heatmap drew");
            let band = ((hot.y - f32::from(HEAT_TOP)) / f32::from(h)).floor() as usize;
            assert_eq!(band, day, "{rows} rows: day {day} is in its own band");
            // Its column: the grid spans LABEL_W..cols-RIGHT_PAD.
            let grid_w = 60.0 - f32::from(LABEL_W) - 2.0;
            let col = ((hot.x - f32::from(LABEL_W)) / grid_w * HOURS as f32).floor() as usize;
            assert_eq!(col, hour, "hour {hour} sits in its own column");
        }
    }
}

#[test]
fn every_day_of_the_week_gets_a_row() {
    let _g = crate::app::theme_test_guard();
    let mut hourly = vec![0u64; DAYS * HOURS];
    for d in 0..DAYS {
        hourly[d * HOURS + 5] = 1_000 * (d as u64 + 1);
    }
    let b = Buckets {
        hourly,
        ..one_hot(0, 0)
    };
    let h = f32::from(layout(40).heat_h);
    let p = paint(&b, 60, 40, 2.0);
    for d in 0..DAYS {
        let band_top = f32::from(HEAT_TOP) + d as f32 * h;
        let any = p
            .iter()
            .any(|r| r.y >= band_top - 0.01 && r.y < band_top + h - 0.01);
        assert!(any, "day {d} has a band of cells");
    }
}

#[test]
fn the_labels_name_every_row_of_the_grid() {
    let _g = crate::app::theme_test_guard();
    let c = cells(&one_hot(0, 0), 60, 40);
    let text = |row: u16| -> String {
        let mut v: Vec<_> = c.iter().filter(|c| c.row == row).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    // Each label is centred on the band it names, so a three-row day is
    // not a label with two unlabelled stripes under it.
    let h = layout(40).heat_h;
    let label = |d: u16| text(HEAT_TOP + d * h + (h - 1) / 2);
    assert!(label(0).starts_with("6d"), "{:?}", label(0));
    assert!(
        label(DAYS as u16 - 1).starts_with("now"),
        "the last band is now: {:?}",
        label(DAYS as u16 - 1)
    );
}

#[test]
fn money_never_rounds_a_real_cost_to_nothing() {
    assert_eq!(money(0), "$0.00");
    assert_eq!(money(1_500_000), "$1.50");
    // A session that cost a third of a cent is not free.
    assert_eq!(money(3_400), "$0.003");
}

/// The total written in the ring's hole must sit INSIDE the hole. It used
/// to be centred on canvas column 6 while the ring was painted on a canvas
/// shifted one column right of that, so at four characters — which is what
/// `compact` is built to produce — its first one landed on the ring's left
/// arc.
#[test]
fn the_total_sits_inside_the_hole_not_on_the_ring() {
    let _g = crate::app::theme_test_guard();
    let b = Buckets {
        hourly: vec![0; DAYS * HOURS],
        daily_cost: vec![0; DAYS],
        tok_in: 1_840_000,
        tok_out: 410_000,
        cost_microusd: 10,
    };
    let l = layout(40);
    let mut hole: Vec<(u16, char)> = cells(&b, 60, 40)
        .into_iter()
        .filter(|c| c.row == l.split_top + RING_ROW && c.col < 13)
        .map(|c| (c.col, c.c))
        .collect();
    hole.sort_by_key(|&(col, _)| col);
    let text: String = hole.iter().map(|&(_, c)| c).collect();
    assert_eq!(text, compact(2_250_000), "the total is in the hole");
    // The centre is measured off the RING ITSELF — the horizontal extent
    // of what `paint` actually emitted in the band — rather than read back
    // out of the same constant the text was placed from. Both ends
    // agreeing with one constant is not the property under test; both ends
    // agreeing with EACH OTHER is, and that is what a stray shift breaks.
    let band: Vec<_> = paint(&b, 60, 40, 2.0)
        .into_iter()
        .filter(|p| p.y >= f32::from(l.split_top) && p.y < f32::from(l.cost_top))
        .collect();
    assert!(!band.is_empty(), "the ring drew something to measure");
    let left = band.iter().fold(f32::MAX, |a, p| a.min(p.x));
    let right = band.iter().fold(0.0f32, |a, p| a.max(p.x + p.w));
    let centre = (left + right) / 2.0;
    // A cell is claimed from its left edge, so the character at `col`
    // spans `col..col+1`; both of its edges must clear the inner wall.
    for &(col, _) in &hole {
        let lo = f32::from(col) - centre;
        let hi = lo + 1.0;
        assert!(
            lo.abs() < RING_R_IN && hi.abs() < RING_R_IN,
            "column {col} of {text:?} is on the ring, not in its hole: \
             the ring runs {left}..{right}, so the hole spans {RING_R_IN} \
             either side of {centre}"
        );
    }
}

/// The ring is nearly two rows tall in each direction. Painted from the
/// legend's own row it put its top arc through the word TOKENS; it starts
/// on the row below now, and the band grew a row to hold it.
#[test]
fn the_ring_never_reaches_the_legend_that_names_it() {
    let _g = crate::app::theme_test_guard();
    let b = Buckets {
        hourly: vec![0; DAYS * HOURS],
        daily_cost: vec![0; DAYS],
        tok_in: 3,
        tok_out: 1,
        cost_microusd: 10,
    };
    let l = layout(40);
    for aspect in [1.6f32, 2.0, 2.4] {
        let ring: Vec<_> = paint(&b, 60, 40, aspect)
            .into_iter()
            .filter(|p| p.y >= f32::from(l.split_top) && p.y < f32::from(l.cost_top))
            .collect();
        assert!(!ring.is_empty(), "the ring drew something at {aspect}");
        let top = ring.iter().fold(f32::MAX, |a, p| a.min(p.y));
        assert!(
            top >= f32::from(l.split_top + 1),
            "the ring is on the legend row at aspect {aspect}: top {top}"
        );
        let bottom = ring.iter().fold(0.0f32, |a, p| a.max(p.y + p.h));
        assert!(
            bottom <= f32::from(l.split_top + SPLIT_ROWS) + 1e-3,
            "and stays in its band at aspect {aspect}: bottom {bottom}"
        );
    }
}

/// A tall pane spends its height on the charts rather than finishing 45%
/// of the way down and leaving the rest paper — the same rule the left
/// nav's own division follows.
#[test]
fn a_tall_pane_gives_its_slack_to_the_charts() {
    let short = layout(24);
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
    // …but neither runs away with it: a week of readings spread over
    // forty rows is a blob, not a shape.
    let huge = layout(200);
    assert_eq!(huge.heat_h, HEAT_ROW_MAX);
    assert_eq!(huge.cost_rows, COST_MAX);
}

/// Every band stays inside the pane, at every height a tile can be — a
/// chart drawn past the last row is a chart drawn over the pane below it.
#[test]
fn no_band_is_ever_laid_out_past_the_last_row() {
    for rows in 0..120u16 {
        let l = layout(rows);
        assert!((1..=HEAT_ROW_MAX).contains(&l.heat_h), "{rows}: {l:?}");
        if l.cost_rows > 0 {
            assert!(l.cost_rows >= COST_MIN, "{rows}: {l:?}");
            // legend + chart + the axis row under it
            assert!(
                l.cost_top + 1 + l.cost_rows < rows,
                "{rows}: the axis row is off the pane: {l:?}"
            );
        }
    }
}

/// The cost band used to ask for five rows or nothing, so a quarter tile
/// dropped it whole and left the rows it could not fill empty. It shrinks
/// to its floor now and is only given up when even that will not fit.
#[test]
fn a_short_pane_shrinks_the_cost_band_before_it_drops_it() {
    // 22 rows is the pane's own floor: every band at its minimum, no
    // slack for anything to grow into.
    let l = layout(22);
    assert!(
        l.cost_rows > 0,
        "a 22-row pane still costs something: {l:?}"
    );
    assert_eq!(l.cost_rows, COST_MIN, "at its floor: {l:?}");
    // And a pane with no room under the donut at all gives it up rather
    // than drawing a one-row smear with an axis on top of it.
    assert_eq!(layout(l.cost_top + 2).cost_rows, 0);
}

/// Both ends of the division agree: the text under the cost curve sits on
/// the row the curve actually stops at. They were two sums before.
#[test]
fn the_cost_axis_labels_sit_under_the_curve_they_label() {
    let _g = crate::app::theme_test_guard();
    let b = Buckets {
        hourly: vec![0; DAYS * HOURS],
        daily_cost: vec![1, 4, 2, 9, 3, 6, 5],
        tok_in: 10,
        tok_out: 5,
        cost_microusd: 100,
    };
    for rows in [24u16, 34, 56] {
        let l = layout(rows);
        let curve: Vec<_> = paint(&b, 60, rows, 2.0)
            .into_iter()
            .filter(|p| p.y >= f32::from(l.cost_top))
            .collect();
        assert!(!curve.is_empty(), "{rows} rows: the curve drew something");
        let bottom = curve.iter().fold(0.0f32, |a, p| a.max(p.y + p.h));
        let axis: Vec<_> = cells(&b, 60, rows)
            .into_iter()
            .filter(|c| c.row >= l.cost_top && c.c == '6')
            .map(|c| c.row)
            .collect();
        let axis_row = l.cost_top + 1 + l.cost_rows;
        assert_eq!(axis.first(), Some(&axis_row));
        // The curve's last quad is a fraction of a cell tall, so it stops
        // just inside the axis row rather than exactly on it — what must
        // not happen is stopping a whole row short, or running past.
        assert!(
            bottom <= f32::from(axis_row) + 1e-3 && bottom > f32::from(axis_row) - 1.0,
            "{rows} rows: the curve stops at {bottom}, the axis is on row {axis_row}"
        );
    }
}

#[test]
fn compact_tokens_fit_in_a_donuts_hole() {
    assert_eq!(compact(0), "0");
    assert_eq!(compact(184_000), "184k");
    assert_eq!(compact(2_250_000), "2.2M");
    assert!(compact(u64::MAX / 2).len() <= 12);
}
