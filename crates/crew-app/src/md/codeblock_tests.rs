use super::super::{render, LineKind, MdLine};
use super::*;

fn flat(l: &MdLine) -> String {
    l.spans.iter().map(|s| s.text.as_str()).collect()
}

fn code(src: &str, cols: usize) -> Vec<String> {
    render(src, cols)
        .iter()
        .filter(|l| l.kind == LineKind::Code)
        .map(flat)
        .collect()
}

/// The chrome: a language label, the code, and the blank row that closes the
/// field.
#[test]
fn code_block_chrome_lines() {
    let lines = render("```rust\nfn x() {}\n```", 40);
    let kinds: Vec<LineKind> = lines.iter().map(|l| l.kind).collect();
    assert_eq!(
        kinds,
        vec![LineKind::CodeHeader, LineKind::Code, LineKind::CodeFooter]
    );
    assert_eq!(flat(&lines[0]), "rust");
    assert_eq!(flat(&lines[1]), "fn x() {}");
    assert_eq!(flat(&lines[2]), "");
}

/// Nothing of the source is lost to the wrap — the marks are added, not
/// substituted.
#[test]
fn a_wrapped_line_keeps_every_character() {
    let rows = code("```\nlet a = 1;\n```", 6);
    let text: String = rows.concat().replace("\u{21aa} ", "");
    assert_eq!(text, "let a = 1;");
}

/// A continued row says it is continued. Without the mark, the tail of a long
/// line reads as the next statement.
#[test]
fn a_continued_row_wears_the_mark_and_the_first_does_not() {
    let rows = code("```\nlet a = 1;\n```", 8);
    assert!(rows.len() > 1, "{rows:?}");
    assert!(!rows[0].starts_with('\u{21aa}'), "{rows:?}");
    for r in &rows[1..] {
        assert!(r.starts_with("\u{21aa} "), "{rows:?}");
    }
}

/// The mark costs the row two columns, and no row is wider than the field.
#[test]
fn no_row_is_wider_than_the_field() {
    for cols in [3usize, 4, 6, 12, 20] {
        for r in code(
            "```\nlet answer = compute(41) + 1; // close enough\n```",
            cols,
        ) {
            let field = cols.saturating_sub(crate::chatfield::PAD * 2).max(1);
            assert!(
                crate::chatwidth::str_w(&r) <= field,
                "{r:?} is wider than {field} at cols={cols}"
            );
        }
    }
}

/// Wide characters are two columns each: cutting by CHARACTER ran a CJK
/// comment past the tinted field it is laid into.
#[test]
fn wide_characters_are_measured_not_counted() {
    let rows = code(
        "```\n// \u{6e2c}\u{8a66}\u{6e2c}\u{8a66}\u{6e2c}\u{8a66}\n```",
        12,
    );
    for r in &rows {
        assert!(crate::chatwidth::str_w(r) <= 10, "{r:?}");
    }
    let text: String = rows.concat().replace("\u{21aa} ", "");
    assert_eq!(text, "// \u{6e2c}\u{8a66}\u{6e2c}\u{8a66}\u{6e2c}\u{8a66}");
}

/// A field with no room for both keeps the code: the mark is the thing that
/// can be inferred, the characters are not.
#[test]
fn a_field_too_narrow_for_the_mark_still_carries_the_code() {
    let rows = code("```\nabcdef\n```", 4);
    assert_eq!(rows.concat().replace("\u{21aa} ", ""), "abcdef");
    assert!(rows.len() >= 3, "{rows:?}");
}

/// The mark is not marker ink: `chatfield::field_start` reads leading marker
/// cells as outside the tint, and this one belongs inside it.
#[test]
fn the_mark_stays_inside_the_field() {
    let lines = render("```\nlet a = 1;\n```", 8);
    let cont = lines
        .iter()
        .filter(|l| l.kind == LineKind::Code)
        .nth(1)
        .expect("a continued row");
    let mark = &cont.spans[0];
    assert_eq!(mark.text, "\u{21aa} ");
    assert!(!mark.style.marker, "the mark would push the field right");
    assert_eq!(mark.style.token, Token::Comment);
}

/// A token that straddles the wrap keeps its colour on both rows.
#[test]
fn a_string_across_a_wrap_stays_one_colour() {
    let lines = render("```rust\nlet s = \"a long string literal\";\n```", 16);
    let rows: Vec<&MdLine> = lines.iter().filter(|l| l.kind == LineKind::Code).collect();
    assert!(rows.len() > 1);
    for r in &rows[1..] {
        assert!(
            r.spans.iter().any(|s| s.style.token == Token::Str),
            "{:?}",
            flat(r)
        );
    }
}

/// The header clips to the field like everything else: a long language tag on
/// a narrow card must not draw past it.
#[test]
fn code_chrome_lines_respect_cols() {
    for cols in [1usize, 4, 6] {
        for l in render("```averylonglanguagetag\nx\n```", cols) {
            assert!(
                crate::chatwidth::str_w(&flat(&l)) <= cols,
                "kind {:?} overflows at cols={cols}: {:?}",
                l.kind,
                flat(&l)
            );
        }
    }
}
