use super::*;

#[test]
fn zero_usage_has_no_trailer() {
    // All-zero stats are real (CLI backends, error hops) — no trailer.
    assert_eq!(trailer_text(0, 0, 0), None);
    assert!(trailer_line((0, 0, 0)).is_none());
}

#[test]
fn trailer_formats_tokens_and_cost_like_the_footer() {
    assert_eq!(
        trailer_text(900, 50, 12_000).as_deref(),
        Some("900 in / 50 out \u{00b7} $0.012")
    );
}

#[test]
fn big_counts_use_the_footer_k_format() {
    // 12_300 in → "12.3k", matching `chathdr::fmt_tokens`; an unpriced model
    // (zero cost, real tokens) keeps its split and drops the cost segment.
    assert_eq!(
        trailer_text(12_300, 5_000, 0).as_deref(),
        Some("12.3k in / 5.0k out")
    );
}

#[test]
fn trailer_line_is_muted_and_indented_like_the_body() {
    let line = trailer_line((900, 50, 12_000)).expect("usage renders a line");
    let text: String = line.iter().map(|c| c.c).collect();
    assert_eq!(text, " 900 in / 50 out \u{00b7} $0.012");
    let muted = crew_theme::theme().text_muted;
    assert!(
        line.iter().all(|c| c.fg == muted && !c.bold),
        "the trailer must render entirely in the muted ink"
    );
}
