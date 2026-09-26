//! `fmt_tokens` tiers, and the header row's spacing.
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

/// A header segment that opens with its own `·` sits one space after the
/// segment before it, like every `a · b` in crew — never `thinking  · esc`.
#[test]
fn a_dotted_segment_is_one_space_off_its_neighbour() {
    let cells = super::header_cells(80, "c", true, true, None, true, 0);
    let mut v: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
    v.sort_by_key(|c| c.col);
    let mut line = String::new();
    let mut at = v.first().map_or(0, |c| c.col);
    for c in v {
        line.extend(std::iter::repeat_n(' ', usize::from(c.col - at)));
        line.push(c.c);
        at = c.col + 1;
    }
    assert!(
        line.contains("thinking \u{b7} compact \u{b7} esc interrupts"),
        "{line:?}"
    );
    assert!(!line.contains("  \u{b7}"), "{line:?}");
}
