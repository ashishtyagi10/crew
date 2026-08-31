use super::*;

#[test]
fn lowercases_and_hyphenates_whitespace() {
    assert_eq!(slug("Risk Assessor").as_deref(), Some("risk-assessor"));
    assert_eq!(slug("  Archivist  ").as_deref(), Some("archivist"));
}

#[test]
fn strips_chars_that_break_the_at_tokenizers() {
    // `@` would double-parse, `+` is relay.rs's multi-target separator,
    // `/` collides with construct routing in stdio.rs.
    assert_eq!(slug("@archivist").as_deref(), Some("archivist"));
    assert_eq!(slug("data+ops").as_deref(), Some("data-ops"));
    assert_eq!(slug("sec/ops").as_deref(), Some("sec-ops"));
    assert_eq!(slug("The Skeptic!").as_deref(), Some("the-skeptic"));
}

#[test]
fn collapses_and_trims_hyphens() {
    assert_eq!(slug("a---b").as_deref(), Some("a-b"));
    assert_eq!(slug("--edge--").as_deref(), Some("edge"));
}

#[test]
fn rejects_what_cannot_be_salvaged() {
    assert_eq!(slug(""), None);
    assert_eq!(slug("@#$"), None);
    assert_eq!(slug("x"), None, "one char is below the floor");
    assert_eq!(slug("---"), None);
}

#[test]
fn non_ascii_is_dropped_not_transliterated() {
    // chips_on_border measures with byte length, which is only correct
    // because the charset is ASCII.
    assert_eq!(slug("café-critic").as_deref(), Some("caf-critic"));
    assert_eq!(slug("日本語"), None);
}

#[test]
fn over_length_is_hard_cut_not_boundary_cut() {
    // A boundary cut would yield "accommodation" — a bare topic noun,
    // the exact failure the planner prompt exists to prevent. A hard cut
    // is obviously mangled instead of plausibly wrong.
    let long = "accommodation-specialist-for-travel";
    let got = slug(long).unwrap();
    assert_eq!(got.len(), 28);
    assert_eq!(got, "accommodation-specialist-for");
}

#[test]
fn hard_cut_still_trims_a_trailing_hyphen() {
    // 30 chars: the cut at 28 lands exactly on the hyphen at index 27, so
    // this fails if the trailing-hyphen trim is ever dropped.
    let got = slug("abcdefghijklmnopqrstuvwxyza-bc").unwrap();
    assert!(!got.ends_with('-'), "got {got}");
    assert_eq!(got, "abcdefghijklmnopqrstuvwxyza");
}

#[test]
fn slug_or_derives_from_id_when_unsalvageable() {
    assert_eq!(slug_or("@#$", 3), "specialist-3");
    assert_eq!(slug_or("Archivist", 3), "archivist");
}

#[test]
fn role_clamp_collapses_whitespace_and_drops_controls() {
    assert_eq!(role_clamp("  records,\n retrieval  "), "records, retrieval");
    assert_eq!(role_clamp("a\u{7}b"), "ab");
    assert_eq!(role_clamp(""), "");
}

#[test]
fn role_clamp_treats_control_whitespace_as_a_separator() {
    assert_eq!(role_clamp("foo\tbar"), "foo bar");
    assert_eq!(role_clamp("foo\r\nbar"), "foo bar");
    assert_eq!(
        role_clamp("a\u{7}b"),
        "ab",
        "a non-whitespace control is dropped, not spaced"
    );
}

#[test]
fn role_clamp_truncates_at_sixty_chars() {
    let got = role_clamp(&"x".repeat(100));
    assert_eq!(got.chars().count(), 60);
}
