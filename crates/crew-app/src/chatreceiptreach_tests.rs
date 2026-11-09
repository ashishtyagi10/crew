//! The reach read off a receipt's head and drawn on its first row.
use super::split;
use crate::chatbody::CardLine;
use crate::chatreceipthead::rows;
use crate::chatreceiptparse::parse;

const ONE: &str = "task #5: 1 file changed (+1 \u{2212}1): ~ src/lib.rs \u{b7} reaches 2 callers in 2 files\n```diff\n-a\n+b\n```";

fn texts(rows: &[CardLine]) -> Vec<String> {
    rows.iter()
        .map(|r| r.iter().map(|c| c.c).collect::<String>())
        .collect()
}

#[test]
fn the_reach_is_read_off_the_head_not_left_on_the_path() {
    let h = parse(ONE.lines().next().unwrap()).unwrap();
    assert_eq!(h.reach, Some((2, 2)));
    assert_eq!(h.named, vec![("~", "src/lib.rs")]);
    let line = "task #7 stopped \u{b7} 2 files changed: ~ a.rs, ~ b.rs \u{b7} reaches 1 caller in 1 file \u{2014} /diff shows them";
    let h = parse(line).unwrap();
    assert_eq!((h.ending, h.reach), (Some("stopped"), Some((1, 1))));
    assert_eq!(h.named[1], ("~", "b.rs"));
    assert_eq!(h.hint, Some("/diff shows them"));
    assert_eq!(
        parse("task #2: 1 file changed: + a.rs").unwrap().reach,
        None
    );
    assert_eq!(
        split("~ a.rs \u{b7} reaches many callers"),
        ("~ a.rs \u{b7} reaches many callers", None)
    );
}

#[test]
fn the_reach_shares_the_heads_row_its_count_in_ink() {
    let _g = crate::app::theme_test_guard();
    let fg = (200, 200, 200);
    let got = rows(ONE, 80, fg).unwrap();
    let t = texts(&got);
    assert_eq!(t.len(), 1, "no row of its own where it fits: {t:#?}");
    assert!(t[0].contains("src/lib.rs  +1 \u{2212}1"), "{}", t[0]);
    assert!(
        t[0].ends_with(" \u{b7} reaches 2 callers in 2 files"),
        "{}",
        t[0]
    );
    let at = t[0]
        .find("2 callers")
        .map(|b| t[0][..b].chars().count())
        .unwrap();
    let muted = crew_theme::theme().text_muted;
    assert_eq!(got[0][at].fg, fg, "the count reads in the card's ink");
    assert_eq!(got[0][at - 2].fg, muted, "the words around it are muted");
    let quarter = texts(&rows(ONE, 40, fg).unwrap());
    assert_eq!(
        quarter.len(),
        1,
        "a quarter tile keeps the count: {quarter:#?}"
    );
    assert!(quarter[0].ends_with(" \u{b7} 2 callers"), "{quarter:#?}");
    let narrow = texts(&rows(ONE, 26, fg).unwrap());
    assert_eq!(narrow.len(), 2, "{narrow:#?}");
    assert!(
        narrow[1].contains("reaches 2 callers in 2 files"),
        "{narrow:#?}"
    );
}
