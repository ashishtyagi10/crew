//! A wrapped line of code hangs under its own indent.
use super::{painted, GUTTER_W};

/// The text of a row past the gutter.
fn body(line: &crate::chatbody::CardLine) -> String {
    line.iter().skip(GUTTER_W).map(|c| c.c).collect()
}

/// "costs the same…" started flush at the gutter under an indented line,
/// reading as a new statement. A continuation starts at the line's indent,
/// every row still fits, and nothing of the line is lost.
#[test]
fn a_wrapped_line_hangs_under_its_indent() {
    let text = "fn f() {\n    let total = first_argument + second_argument + third_argument;\n}";
    let cols = GUTTER_W + 28;
    let (lines, src) = painted(text, cols, &[], (200, 200, 200), (120, 120, 120));
    let rows: Vec<usize> = (0..lines.len()).filter(|&r| src[r] == 1).collect();
    assert!(rows.len() > 1, "the long line wraps");
    for &r in &rows[1..] {
        let b = body(&lines[r]);
        assert!(b.starts_with("    ") && !b.starts_with("     "), "{b:?}");
    }
    for l in &lines {
        assert!(l.len() <= cols, "a row overran: {}", l.len());
    }
    let joined: String = rows
        .iter()
        .map(|&r| body(&lines[r]).trim().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        joined,
        "let total = first_argument + second_argument + third_argument;"
    );
}
