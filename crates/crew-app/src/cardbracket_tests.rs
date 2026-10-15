//! The focus brackets' two arms, against the frame's stretched last row.
use super::even_brackets;
use crate::panecard::Bar;

/// The focus brackets come out even: with more than half a row of the
/// card's remainder stretched under its last row, the bottom arm gives its
/// innermost row back to the stroke (glass survey G7: 74 px against 62).
#[test]
fn a_long_remainder_takes_a_row_off_the_bottom_bracket() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let b = Bar {
        focused: true,
        focus_t: 1.0,
        assemble_t: 1.0,
        ..Default::default()
    };
    let (cols, rows) = (40u16, 30u16);
    let n = usize::from(crate::cardchrome::bracket_len(rows, 1.0));
    let card = crate::panecard::pane_card(cols - 2, rows - 2, &b);
    let lit = |v: &[crew_render::CellView], rs: std::ops::Range<u16>| {
        rs.filter(|&r| v.iter().any(|c| c.col == 0 && c.row == r && c.bold))
            .count()
    };
    for (long, bottom) in [(false, n), (true, n - 1)] {
        let mut v = card.clone();
        even_brackets(&mut v, (cols, rows), &b, long);
        assert_eq!(lit(&v, 1..rows / 2), n, "top arm, long={long}");
        assert_eq!(
            lit(&v, rows / 2..rows - 1),
            bottom,
            "bottom arm, long={long}"
        );
    }
}
