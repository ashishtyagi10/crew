use super::*;

fn text_at(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    let mut out = String::new();
    let mut col = 0;
    for c in v {
        while col < c.col {
            out.push(' ');
            col += 1;
        }
        out.push(c.c);
        col += 1;
    }
    out
}

/// The ticks sit under the hours they name: midnight at the grid's left edge,
/// noon at its middle.
#[test]
fn the_hour_ticks_land_on_their_own_columns() {
    let _g = crate::app::theme_test_guard();
    let mut out = Vec::new();
    hour_ticks(&mut out, 4, 48, 9, 80);
    let row = text_at(&out, 9);
    assert_eq!(row.trim_end(), "    00          06          12          18");
    assert!(out.iter().all(|c| c.row == 9));
}

/// A grid narrower than the ticks still puts them in order and inside the
/// pane — a tick drawn past the card is a tick on somebody else's chart.
#[test]
fn narrow_grids_keep_their_ticks_inside_the_pane() {
    let _g = crate::app::theme_test_guard();
    for cols in [24u16, 30, 46] {
        let mut out = Vec::new();
        hour_ticks(&mut out, 4, cols - 6, 3, cols);
        assert!(out.iter().all(|c| c.col < cols), "{cols}: a tick escaped");
    }
}

#[test]
fn the_week_is_named_at_both_ends() {
    let _g = crate::app::theme_test_guard();
    let mut out = Vec::new();
    week_ends(&mut out, 4, 40, 2);
    let row = text_at(&out, 4);
    assert!(row.starts_with(" 6d ago"), "{row:?}");
    assert!(row.trim_end().ends_with("today"), "{row:?}");
}

/// The two shares of the ring add to 100. Flooring each on its own gave
/// `81% / 18%` for a 1.92M/0.43M split.
#[test]
fn the_two_shares_add_up() {
    assert_eq!(split_pct(1_920_000, 430_000), (82, 18));
    for (a, b) in [(1u64, 2u64), (1, 1), (999, 1), (7, 993), (0, 5), (5, 0)] {
        let (pa, pb) = split_pct(a, b);
        assert_eq!(pa + pb, 100, "{a}/{b} gave {pa}+{pb}");
    }
}

#[test]
fn nothing_spent_is_no_share_at_all() {
    assert_eq!(split_pct(0, 0), (0, 0));
}

/// The point that is left over goes to the share it was taken from — the one
/// whose remainder was bigger, not simply the first.
#[test]
fn the_leftover_point_goes_to_the_larger_remainder() {
    // 1/3 → 33.33 / 66.67: the out share is the one rounding up.
    assert_eq!(split_pct(1, 2), (33, 67));
    assert_eq!(split_pct(2, 1), (67, 33));
}
