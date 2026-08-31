use crate::chatbody::body_lines;

fn text(line: &crate::chatbody::CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

/// The field is a rectangle: every row of the block the same width,
/// whatever the longest line is. A background that stopped where each
/// line did was the ragged edge this replaced.
#[test]
fn every_row_of_a_block_is_the_same_width() {
    let _g = crate::app::theme_test_guard();
    let lines = body_lines("```rs\nx\nlonger line here\ny\n```", 40, (9, 9, 9), false);
    let widths: Vec<usize> = lines.iter().map(|l| text(l).chars().count()).collect();
    assert!(
        widths.windows(2).all(|w| w[0] == w[1]),
        "{:?}",
        lines.iter().map(text).collect::<Vec<_>>()
    );
    // Widest line (16) + a pad each side + the card's indent column.
    assert_eq!(widths[0], 16 + super::PAD * 2 + 1);
}

/// A fence inside a blockquote keeps its bar on the page. The bar is
/// prefixed to every line of the quote, code rows included, and a field
/// that started at column 1 would eat it.
#[test]
fn a_quoted_fence_keeps_its_bar_off_the_field() {
    let _g = crate::app::theme_test_guard();
    let lines = body_lines("> ```\n> x = 1\n> ```", 40, (9, 9, 9), false);
    let bg = Some(crate::chatink::code_bg());
    let starts: Vec<usize> = lines
        .iter()
        .map(|l| l.iter().position(|c| c.bg == bg).expect("a field"))
        .collect();
    for (line, start) in lines.iter().zip(&starts) {
        assert!(*start >= 2, "the bar stays on the page: {:?}", text(line));
        assert!(line[..*start].iter().all(|c| c.bg.is_none()));
    }
    assert!(
        starts.windows(2).all(|w| w[0] == w[1]),
        "the field's left edge is one column on every row: {starts:?}"
    );
}
