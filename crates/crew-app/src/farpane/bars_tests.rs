use super::prompt_text;
use crate::chatwidth::str_w;

#[test]
fn a_short_name_sits_after_its_label_with_the_caret() {
    assert_eq!(
        prompt_text("Create folder: ", "shots", 40),
        "Create folder: shots\u{258f}"
    );
}

/// The row is where you watch what you type. When the name outgrows it, the
/// end of the name — and the caret — stay; the start goes behind a mark.
#[test]
fn a_long_name_keeps_its_end_and_its_caret_in_view() {
    let name = "screenshots-from-the-second-of-september-before-the-release";
    let row = prompt_text("Create folder: ", name, 30);
    assert_eq!(str_w(&row), 30, "{row:?}");
    assert!(row.starts_with("Create folder: \u{2026}"), "{row:?}");
    assert!(row.ends_with("the-release\u{258f}"), "{row:?}");
}

#[test]
fn a_wide_glyph_never_straddles_the_mark() {
    let row = prompt_text(
        "Create folder: ",
        "\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30d5}\u{30a9}\u{30eb}\u{30c0}",
        24,
    );
    assert!(str_w(&row) <= 24, "{row:?}");
    assert!(row.ends_with('\u{258f}'));
}
