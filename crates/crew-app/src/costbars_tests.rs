use super::*;

fn text_at(out: &[CellView], row: u16) -> String {
    let mut v: Vec<&CellView> = out.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    let mut s = String::new();
    let mut col = 0;
    for c in v {
        while col < c.col {
            s.push(' ');
            col += 1;
        }
        s.push(c.c);
        col += 1;
    }
    s
}

const WEEK: [u64; 7] = [130_000, 420_000, 0, 40_000, 900_000, 510_000, 280_000];

#[test]
fn every_day_is_named_and_every_spend_says_what_it_was() {
    let _g = crate::app::theme_test_guard();
    let mut out = Vec::new();
    labels(&mut out, &WEEK, 1, 98, 10, 6, 16, 100);
    let axis: Vec<String> = text_at(&out, 16)
        .split_whitespace()
        .map(str::to_string)
        .collect();
    assert_eq!(axis, ["6d", "5d", "4d", "3d", "2d", "1d", "now"]);
    let all: String = (10..16)
        .map(|r| text_at(&out, r))
        .collect::<Vec<_>>()
        .join("\n");
    for v in ["$0.13", "$0.42", "$0.04", "$0.90", "$0.51", "$0.28"] {
        assert!(all.contains(v), "{v} missing from\n{all}");
    }
    // A day that cost nothing is its stub, not a `$0.00` over nothing.
    assert!(!all.contains("$0.00"), "{all}");
}

#[test]
fn the_peak_day_is_labelled_on_the_rows_the_chart_owns() {
    let _g = crate::app::theme_test_guard();
    let mut out = Vec::new();
    labels(&mut out, &WEEK, 1, 98, 10, 6, 16, 100);
    assert!(
        text_at(&out, 10).contains("$0.90"),
        "{:?}",
        text_at(&out, 10)
    );
    assert!(
        out.iter().all(|c| c.row >= 10),
        "a label escaped above the chart"
    );
}

#[test]
fn a_narrow_chart_names_the_ends_instead() {
    let _g = crate::app::theme_test_guard();
    let mut out = Vec::new();
    labels(&mut out, &WEEK, 1, 20, 10, 6, 16, 22);
    let axis = text_at(&out, 16);
    assert!(
        axis.contains("6d ago") && axis.contains("today"),
        "{axis:?}"
    );
    // And no value is jammed into a slot it does not fit.
    assert!((10..16).all(|r| !text_at(&out, r).contains('$')));
}

#[test]
fn the_bars_leave_the_top_row_for_the_labels() {
    let _g = crate::app::theme_test_guard();
    let p = paint(&WEEK, 70, 5, 2.0);
    assert!(!p.is_empty());
    assert!(
        p.iter().all(|p| p.y >= 1.0 - 1e-3),
        "a bar reached the label row"
    );
}
