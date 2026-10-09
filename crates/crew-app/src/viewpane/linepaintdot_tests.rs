//! A wrapped listing never ends a row on its separator dot.
use super::wrap;

fn rows(text: &str, w: usize) -> Vec<String> {
    wrap(text, w, super::super::rowcut::hang)
        .into_iter()
        .map(|(_, r)| r.into_iter().collect())
        .collect()
}

#[test]
fn a_dot_left_at_a_row_end_is_blanked() {
    let line = "/watching cancel <id> calls one off \u{b7} /watching snooze <id> 30m";
    let r = rows(line, 38);
    assert!(r.len() > 1, "{r:?}");
    assert!(!r[0].trim_end().ends_with('\u{b7}'), "{r:?}");
    // Paint only: the rows still partition the line, char for char.
    let lens: usize = wrap(line, 38, super::super::rowcut::hang)
        .iter()
        .map(|(_, r)| r.len())
        .sum();
    assert_eq!(lens, line.chars().count());
}

#[test]
fn a_dot_inside_a_row_or_ending_the_line_stays() {
    let r = rows("in 2h \u{b7} daily", 80);
    assert_eq!(r, ["in 2h \u{b7} daily"]);
    assert_eq!(
        rows("done \u{b7}", 80),
        ["done \u{b7}"],
        "the line's own end is content"
    );
    // A dot glued to a word is not a separator.
    let r = rows("aaaa bbbb\u{b7} cccc dddd", 11);
    assert!(r[0].contains('\u{b7}'), "{r:?}");
}
