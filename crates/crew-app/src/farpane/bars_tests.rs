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

/// Pills leave from the right, Quit last of all; the bar never stops
/// mid-pill.
#[test]
fn the_function_bar_drops_whole_pills_and_keeps_quit() {
    use super::pills_that_fit;
    let all = pills_that_fit(120);
    assert_eq!(all.len(), 8);
    let narrow = pills_that_fit(50);
    assert!(narrow.len() < 8 && narrow.len() > 1, "{narrow:?}");
    assert_eq!(narrow.last(), Some(&("10", "Quit")));
    assert_eq!(narrow[0], ("1", "Help"), "drops from the right");
    let width = |p: &[(&str, &str)]| p.iter().map(|(k, l)| k.len() + l.len() + 5).sum::<usize>();
    assert!(width(&narrow) <= 50, "{}", width(&narrow));
    assert_eq!(pills_that_fit(4), vec![("10", "Quit")]);
}

/// The status row measures the way the prompt under it does.
#[test]
fn the_status_row_ellipsizes_by_display_width() {
    use super::ellipsize_keeping_suffix;
    let s = ellipsize_keeping_suffix("日本語のフォルダの名前 \u{b7} 1.2M", 14);
    assert!(str_w(&s) <= 14, "{s:?}");
    assert!(s.ends_with(" \u{b7} 1.2M"), "{s:?}");
    assert!(s.contains('\u{2026}'));
    assert_eq!(
        ellipsize_keeping_suffix("short \u{b7} 1K", 40),
        "short \u{b7} 1K"
    );
}
