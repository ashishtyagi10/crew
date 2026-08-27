use super::*;

use crate::chatbody::plain;

fn cardline(s: &str) -> CardLine {
    s.chars()
        .map(|c| plain(c, crew_theme::theme().ink, false))
        .collect()
}

fn text_of(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

#[test]
fn each_row_is_labelled_by_the_source_line_its_gutter_names() {
    let mut lines = vec![
        cardline("    1 fn main() {"),
        cardline("    2     work();"),
        cardline("    3 }"),
    ];
    let labels = vec![
        "aaa1111 Ada".into(),
        "           ".into(),
        "bbb2222 Bob".into(),
    ];
    apply(&mut lines, &labels, 11);
    assert_eq!(text_of(&lines[0]), "aaa1111 Ada    1 fn main() {");
    assert_eq!(text_of(&lines[1]), "               2     work();");
    assert_eq!(text_of(&lines[2]), "bbb2222 Bob    3 }");
}

/// A wrapped line's continuation rows carry a `↪` instead of a number, and
/// so belong to no source line of their own: they get blanks, and the text
/// column stays put.
#[test]
fn a_wrap_continuation_gets_blanks_not_the_previous_label() {
    let mut lines = vec![
        cardline("    7 a long line"),
        cardline("    \u{21aa} that wrapped"),
    ];
    let labels = vec![String::new(); 6]
        .into_iter()
        .chain([String::from("ccc3333 Cee")])
        .collect::<Vec<_>>();
    apply(&mut lines, &labels, 11);
    assert_eq!(text_of(&lines[0]), "ccc3333 Cee    7 a long line");
    assert_eq!(
        text_of(&lines[1]),
        "               \u{21aa} that wrapped",
        "a continuation is not a second blame row"
    );
}

/// Banners and any other row with no numbered gutter still get their column,
/// so the whole rendering stays one rectangle.
#[test]
fn a_row_with_no_source_line_still_gets_its_column() {
    let mut lines = vec![cardline("loading\u{2026}")];
    apply(&mut lines, &[], 7);
    assert_eq!(text_of(&lines[0]), "       loading\u{2026}");
}

/// A blame shorter than the rendering — the file grew since the read — pads
/// rather than shifting the rows it does not cover. And a label shorter than
/// the column pads too: `labels` hands over pre-padded strings today, but a
/// short one here would step the text left on that row alone and nowhere
/// else, which is the hardest kind of ragged to notice.
#[test]
fn a_short_or_missing_label_still_fills_its_column() {
    let mut lines = vec![
        cardline("    1 old"),
        cardline("    2 mid"),
        cardline("    3 new"),
    ];
    apply(&mut lines, &["ddd4444 Dee".into(), "short".into()], 11);
    assert_eq!(text_of(&lines[0]), "ddd4444 Dee    1 old");
    assert_eq!(
        text_of(&lines[1]),
        "short          2 mid",
        "a short label pads"
    );
    assert_eq!(
        text_of(&lines[2]),
        "               3 new",
        "and a missing one"
    );
    assert!(
        lines.iter().all(|l| l.len() == lines[0].len()),
        "one rectangle"
    );
}

#[test]
fn a_zero_width_column_is_a_strict_no_op() {
    let mut lines = vec![cardline("    1 x")];
    apply(&mut lines, &["ignored".into()], 0);
    assert_eq!(text_of(&lines[0]), "    1 x");
}
