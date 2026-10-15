use super::{away, Doc};
use crate::panecard::{pane_card, Bar};

/// The top border of a 40×20 card over a `total`-row buffer, `scroll` rows
/// back from its bottom, as text.
fn top(doc: Doc, total: usize, scroll: usize) -> String {
    let b = Bar {
        title: "Cargo.toml",
        scroll,
        total,
        doc,
        ..Default::default()
    };
    let mut row: Vec<_> = pane_card(40, 20, &b)
        .into_iter()
        .filter(|c| c.row == 0)
        .collect();
    row.sort_by_key(|c| c.col);
    row.into_iter().map(|c| c.c).collect()
}

/// A viewer opened on line 1 is at home. It used to wear `⇡110` there —
/// rows up from the END — as if it had been scrolled.
#[test]
fn a_file_at_its_top_stamps_no_count() {
    let _g = crate::app::theme_test_guard();
    let border = top(Doc::File, 130, 110);
    assert!(
        !border.contains('\u{21e1}') && !border.contains('\u{21e3}'),
        "a file at its first line carries a count: {border}"
    );
}

#[test]
fn a_file_counts_down_from_its_first_line() {
    let _g = crate::app::theme_test_guard();
    // 130 rows, 20 visible: 40 rows down is 70 back from the bottom.
    let border = top(Doc::File, 130, 70);
    assert!(border.contains("\u{21e3}40"), "{border}");
    // At its last screenful it says how far down that is.
    assert!(top(Doc::File, 130, 0).contains("\u{21e3}110"));
}

/// A shell and a transcript live at their bottom: nothing there, `⇡N` back.
#[test]
fn a_shell_and_a_transcript_still_count_back_from_the_bottom() {
    let _g = crate::app::theme_test_guard();
    for doc in [Doc::No, Doc::Transcript] {
        assert!(!top(doc, 130, 0).contains('\u{21e1}'), "{doc:?} at home");
        let border = top(doc, 130, 12);
        assert!(border.contains("\u{21e1}12"), "{doc:?}: {border}");
    }
}

#[test]
fn away_never_runs_past_the_buffer() {
    // A buffer shorter than its window is at home whatever it reports.
    assert_eq!(away(Doc::File, 5, 20, 0), (0, '\u{21e3}'));
    assert_eq!(away(Doc::File, 130, 20, 500), (0, '\u{21e3}'));
}
