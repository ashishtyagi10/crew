use super::*;

/// `line 1` to `line n`, one per line.
fn file(n: usize) -> String {
    (1..=n).map(|i| format!("line {i}\n")).collect()
}

fn lines(first: usize, last: usize) -> Edit {
    Edit {
        first,
        last,
        cut: 0,
    }
}

#[test]
fn a_200_line_paste_shows_its_head_and_points_at_the_rest() {
    // Lines 10..=209 are the paste, in a file that had 20 lines.
    let head: String = (1..=9).map(|i| format!("line {i}\n")).collect();
    let paste: String = (1..=200).map(|i| format!("new {i}\n")).collect();
    let tail: String = (11..=20).map(|i| format!("line {i}\n")).collect();
    let got = layout(vec![lines(10, 209)], &format!("{head}{paste}{tail}"));
    let rows: Vec<&str> = got.lines().collect();
    assert_eq!(rows.len(), 41, "{got}");
    assert_eq!(rows[0], " 8\u{2502} line 8");
    assert_eq!(rows[39], "47\u{2502} new 38");
    // 10..=47 were shown, so 48..=209 are left: 162 lines.
    assert_eq!(
        rows[40],
        "\u{2026} (+162 more lines edited; sys:read_file with \"line\": 48 shows them)"
    );
}

#[test]
fn long_lines_are_capped_by_chars_not_just_rows() {
    let wide: String = (1..=30)
        .map(|i| format!("{:<150}\n", format!("wide {i}")))
        .collect();
    let got = layout(vec![lines(3, 32)], &format!("a\nb\n{wide}c\nd\n"));
    let rows: Vec<&str> = got.lines().collect();
    let (body, last) = rows.split_at(rows.len() - 1);
    assert!(body.join("\n").chars().count() <= MAX_CHARS, "{got}");
    // Two short rows and twelve 154-char ones fit; the thirteenth does not.
    assert!(
        body.last().unwrap().starts_with("14\u{2502} wide 12 "),
        "{got}"
    );
    assert_eq!(
        last[0],
        "\u{2026} (+18 more lines edited; sys:read_file with \"line\": 15 shows them)"
    );
}

#[test]
fn one_very_long_line_is_cut_where_the_cap_says() {
    let got = layout(vec![lines(1, 1)], &format!("{}\nb\n", "x".repeat(500)));
    let first = got.lines().next().unwrap();
    assert_eq!(first, format!("1\u{2502} {}\u{2026}", "x".repeat(LINE_CAP)));
}

#[test]
fn regions_that_touch_run_together_and_a_line_apart_do_not() {
    // 10 and 15: 8..=12 and 13..=17 touch, so one region.
    let got = layout(vec![lines(10, 10), lines(15, 15)], &file(40));
    assert!(!got.contains('\u{22ee}'), "{got}");
    assert_eq!(got.lines().count(), 10, "{got}");
    // 10 and 16: 8..=12 and 14..=18 leave line 13 out, so two.
    let got = layout(vec![lines(10, 10), lines(16, 16)], &file(40));
    assert!(got.contains("\n \u{22ee}\n14\u{2502} line 14"), "{got}");
    assert!(!got.contains("13\u{2502}"), "{got}");
}

#[test]
fn only_removals_left_past_the_cap_are_pointed_at_as_removals() {
    // Nine one-line removals ten lines apart: five rows and a break each, so
    // six regions fit in forty rows and the last three do not.
    let edits = (1..=9).map(|k| Edit {
        first: 10 * k,
        last: 10 * k - 1,
        cut: 1,
    });
    let got = layout(edits.collect(), &file(100));
    assert!(
        got.contains("\u{2502} \u{2504} 1 line removed\n60\u{2502}"),
        "{got}"
    );
    assert!(!got.contains("\n70\u{2502}"), "{got}");
    assert_eq!(
        got.lines().last().unwrap(),
        "\u{2026} (+3 more lines removed; sys:read_file with \"line\": 69 shows them)"
    );
}

#[test]
fn nothing_to_show_is_an_empty_string() {
    // The whole file taken out mid-line: no line is left to number.
    assert_eq!(layout(vec![lines(1, 1)], ""), "");
}

#[test]
fn a_copied_numbered_row_is_recognised() {
    assert!(carries_numbers(
        "  41\u{2502} let x = 1;\n  42\u{2502} let y = 2;"
    ));
    assert!(carries_numbers("9\u{2502} a\n\n10\u{2502} b"));
    assert!(!carries_numbers("let x = 1;"));
    assert!(!carries_numbers("41 let x = 1;"));
    assert!(!carries_numbers("  41\u{2502} a\nplain line"));
    assert!(!carries_numbers(""));
}
