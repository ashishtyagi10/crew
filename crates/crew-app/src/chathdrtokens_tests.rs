//! `fmt_tokens` tiers.
/// The footer's token count climbs past `k` rather than growing to seven
/// characters, and never rounds up into a tier it has not reached.
#[test]
fn token_counts_climb_to_m_and_g() {
    use super::fmt_tokens;
    assert_eq!(fmt_tokens(950), "950");
    assert_eq!(fmt_tokens(12_300), "12.3k");
    assert_eq!(fmt_tokens(999_949), "999.9k");
    assert_eq!(fmt_tokens(999_950), "1.0M", "not `1000.0k`");
    assert_eq!(fmt_tokens(4_123_500), "4.1M");
    assert_eq!(fmt_tokens(2_250_000_000), "2.2G");
}
