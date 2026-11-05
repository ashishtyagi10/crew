use super::*;
use crate::chatlayout::Message;
use crate::chatmsgs::{full_body, View};
use crate::chatreceiptparse::{parse, per_file};

const ONE: &str = "task #3: 1 file changed (+4 \u{2212}1): ~ crates/crew-theme/src/lib.rs\n\u{2713} checked: `cargo test` passed\n```diff\n-    (r, g, b)\n+    (b, g, r)\n```";

const MANY: &str = "task #7 stopped \u{b7} 5 files changed (+9 \u{2212}3): ~ src/a.rs, + src/b.rs, ~ src/c.rs, ~ src/d.rs, +1 more \u{2014} /diff shows them; \u{201c}undo that\u{201d} puts them back\n```diff\ndiff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n+x\n+y\n-z\ndiff --git a/src/b.rs b/src/b.rs\n--- /dev/null\n+++ b/src/b.rs\n+1\n+2\n+3\n+4\n+5\n+6\ndiff --git a/src/c.rs b/src/c.rs\n-q\n-r\n```";

fn smith(text: &str) -> Message {
    Message {
        sender: "agent smith".into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: true,
    }
}

fn texts(rows: &[CardLine]) -> Vec<String> {
    rows.iter()
        .map(|r| r.iter().map(|c| c.c).collect::<String>())
        .collect()
}

#[test]
fn the_head_parses_back_into_its_parts() {
    let h = parse(MANY.lines().next().unwrap()).unwrap();
    assert_eq!(h.ending, Some("stopped"));
    assert_eq!((h.files, h.total), (5, Some((9, 3))));
    assert_eq!(h.named.len(), 4, "`+1 more` is a count, not a file");
    assert_eq!(h.named[1], ("+", "src/b.rs"));
    assert_eq!(
        h.hint,
        Some("/diff shows them; \u{201c}undo that\u{201d} puts them back")
    );
    assert!(parse("task #2: 1 file changed: + a.rs").is_some());
    assert!(parse("task #2 is a plan").is_none());
    assert_eq!(
        per_file(MANY),
        vec![("src/a.rs", 2, 1), ("src/b.rs", 6, 0), ("src/c.rs", 0, 2)]
    );
}

#[test]
fn a_narrow_receipt_never_splits_a_marker_from_its_path() {
    let _g = crate::app::theme_test_guard();
    // 34 columns: the printed head wrapped as `… (+4 −1): ~` / the path.
    let rows = texts(&full_body(&smith(ONE), 34, View::default()));
    assert!(
        !rows
            .iter()
            .any(|r| r.trim() == "~" || r.trim_end().ends_with(" ~")),
        "{rows:#?}"
    );
    let file = rows.iter().find(|r| r.contains('~')).expect("a file row");
    assert!(file.contains("lib.rs") && file.contains("+4"), "{file}");
    assert!(
        !rows[0].contains("task #"),
        "the task number is not the user's word"
    );
    assert!(
        rows.iter().any(|r| r.contains("checked")),
        "the verdict stays under it"
    );
}

#[test]
fn the_counts_are_drawn_in_the_diffs_inks() {
    let _g = crate::app::theme_test_guard();
    let body = full_body(&smith(ONE), 80, View::default());
    let row = body.iter().find(|r| r.iter().any(|c| c.c == '~')).unwrap();
    let at = |c: char| row.iter().position(|x| x.c == c).unwrap();
    let added = crate::chatink::token_fg(Token::Added);
    let removed = crate::chatink::token_fg(Token::Removed);
    assert_eq!(row[at('4')].fg, added, "+4 in the added ink");
    assert_eq!(row[at('\u{2212}')].fg, removed, "−1 in the removed ink");
    let bar: Vec<_> = row.iter().filter(|c| c.c == CELL).map(|c| c.fg).collect();
    assert_eq!(bar.len(), BAR, "the only file has the whole bar");
    assert_eq!(
        bar,
        [added, added, added, added, removed],
        "4:1, added first"
    );
}

#[test]
fn many_files_take_a_row_each_up_to_three_then_a_count() {
    let _g = crate::app::theme_test_guard();
    let rows = texts(&rows(MANY, 80, (200, 200, 200)).unwrap());
    assert!(
        rows[0].contains("stopped \u{b7} 5 files changed  +9 \u{2212}3 \u{b7} 2 more"),
        "{}",
        rows[0]
    );
    assert!(rows[1].contains("~ src/a.rs  +2 \u{2212}1"), "{}", rows[1]);
    assert!(rows[2].contains("+ src/b.rs  +6 \u{2212}0"), "{}", rows[2]);
    assert!(rows[3].contains("~ src/c.rs  +0 \u{2212}2"), "{}", rows[3]);
    // The bars scale against the largest change (6 lines = 5 cells).
    let bar = |r: &str| r.chars().filter(|c| *c == CELL).count();
    assert_eq!([bar(&rows[1]), bar(&rows[2]), bar(&rows[3])], [3, 5, 2]);
    // The counts line up in one column.
    let col = |r: &str| r.find("  +").unwrap();
    assert_eq!(col(&rows[1]), col(&rows[2]));
}

#[test]
fn a_narrow_row_drops_its_bar_then_the_paths_left_end() {
    let _g = crate::app::theme_test_guard();
    let rows = texts(&rows(ONE, 26, (200, 200, 200)).unwrap());
    assert!(!rows[0].contains(CELL), "{rows:?}");
    assert!(
        rows[0].contains("\u{2026}") && rows[0].contains("lib.rs  +4 \u{2212}1"),
        "{rows:?}"
    );
    assert!(rows[0].chars().count() <= 25, "{rows:?}");
}

#[test]
fn a_five_file_head_takes_four_rows_and_says_everything() {
    let _g = crate::app::theme_test_guard();
    // v0.29.77 drew it in six: lead, three files, `+2 more files`, the tip.
    let got = texts(&rows(MANY, 80, (200, 200, 200)).unwrap());
    assert_eq!(got.len(), 4, "{got:#?}");
    assert!(got[0].contains("5 files changed"), "{}", got[0]);
    assert!(got[0].contains("2 more"), "the unlisted files are counted");
    assert!(
        got[0].ends_with("\u{201c}undo that\u{201d} puts them back"),
        "the tip rides the lead row: {}",
        got[0]
    );
    let muted = crew_theme::theme().text_muted;
    let lead = &rows(MANY, 80, (200, 200, 200)).unwrap()[0];
    let tip = lead.iter().rev().take(4).all(|c| c.fg == muted);
    assert!(tip, "the tip is muted, under the counts' weight");
    // Too narrow for the tip on the lead: it takes a row, never a cut word.
    let narrow = texts(&rows(MANY, 50, (200, 200, 200)).unwrap());
    assert!(
        narrow.iter().any(|r| r.contains("puts them back")),
        "{narrow:#?}"
    );
    assert!(narrow.len() <= 5, "{narrow:#?}");
}

#[test]
fn a_one_file_heads_tip_shares_its_row() {
    let _g = crate::app::theme_test_guard();
    let one = ONE.replacen(
        "src/lib.rs\n",
        "src/lib.rs \u{2014} /diff shows them; \u{201c}undo that\u{201d} puts them back\n",
        1,
    );
    let got = texts(&rows(&one, 80, (200, 200, 200)).unwrap());
    assert_eq!(got.len(), 1, "{got:#?}");
    assert!(got[0].contains("lib.rs  +4"), "{}", got[0]);
    assert!(!got[0].contains("/diff"), "the card holds the diff itself");
}
