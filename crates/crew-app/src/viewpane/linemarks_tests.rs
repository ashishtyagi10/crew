//! The diff rung's trailing-whitespace marks.
use super::diff_lines;

/// The dots a row carries in the bell colour (the trailing-space mark).
fn dots(line: &crate::chatbody::CardLine) -> usize {
    let bell = crew_theme::theme().bell;
    line.iter()
        .filter(|c| c.c == '\u{b7}' && c.fg == bell)
        .count()
}

/// A long added line that wraps carries no mark where it wraps: the break
/// space is the wrap's, not the line's. Its real trailing space, on its last
/// row, still does.
#[test]
fn a_soft_wrap_is_not_trailing_whitespace() {
    let _g = crate::app::theme_test_guard();
    let text = "@@ -1 +1 @@\n+one two three four five six seven eight nine ten eleven  ";
    let (lines, _) = diff_lines(text, 30, &[]);
    assert!(lines.len() > 3, "the added line wraps: {}", lines.len());
    let (body, last) = lines[1..].split_at(lines.len() - 2);
    assert!(body.iter().all(|l| dots(l) == 0), "a wrap was marked");
    assert!(
        dots(&last[0]) > 0,
        "the real trailing space is still marked"
    );
}
