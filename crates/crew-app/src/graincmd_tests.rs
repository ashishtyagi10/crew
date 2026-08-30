use super::{label_of, parse, LADDER, MAX};

#[test]
fn the_ladder_names_run_flattest_first_and_straddle_the_default() {
    let vals: Vec<f32> = LADDER.iter().map(|(_, v, _)| *v).collect();
    assert!(vals.windows(2).all(|w| w[0] < w[1]), "{vals:?}");
    assert_eq!(vals.first(), Some(&0.0), "the ladder must reach flat");
    assert_eq!(
        parse("medium"),
        Some(crate::config::default_paper_grain()),
        "`medium` must be the value a fresh install already has"
    );
    assert!(vals.iter().all(|v| *v <= MAX));
}

#[test]
fn numbers_and_names_both_land_and_out_of_range_clamps() {
    assert_eq!(parse("off"), Some(0.0));
    assert_eq!(
        parse("on"),
        Some(crate::config::default_paper_grain()),
        "`on` is the ladder's middle"
    );
    assert_eq!(parse("0.4"), Some(0.4));
    assert_eq!(parse("9"), Some(MAX), "out of range clamps, never fails");
    assert_eq!(parse("-1"), Some(0.0));
    assert_eq!(parse("chunky"), None);
    assert_eq!(parse(""), None);
}

/// A custom amount must report as its number, not as the nearest name — the
/// same honesty `/smooth 42` keeps.
#[test]
fn labels_name_the_ladder_and_number_the_rest() {
    assert_eq!(label_of(0.0), "off");
    assert_eq!(label_of(1.3), "medium");
    assert_eq!(label_of(0.4), "0.4");
    assert_eq!(label_of(1.0), "1.0", "a custom amount is its number");
}
