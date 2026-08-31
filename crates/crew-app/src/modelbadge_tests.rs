use super::*;

#[test]
fn a_live_sub_cent_rate_never_badges_as_free() {
    // 5_000 µ$/Mtok = $0.000005/Mtok — nonzero, but rounds to "0.00" at
    // two decimals. Must render as a paid row, not "$0/$0" (free-looking).
    assert_eq!(dollars(5_000), "<0.01");
    let badge = price_badge(Some((5_000, 5_000)), false);
    assert_eq!(badge, "$<0.01/$<0.01");
    assert_ne!(badge, "$0/$0");
}

#[test]
fn ordinary_rates_are_unaffected() {
    assert_eq!(dollars(0), "0");
    assert_eq!(dollars(3_000_000), "3");
    assert_eq!(dollars(400_000), "0.4");
    assert_eq!(dollars(1_650_000), "1.65");
    assert_eq!(dollars(5_000_000), "5");
}
