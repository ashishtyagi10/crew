use crate::{FindSpan, GridSize, HeadlessTerm, TermModel};

fn term(cols: u16, rows: u16, text: &str) -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols, rows });
    t.feed(text.as_bytes());
    t
}

#[test]
fn matches_run_newest_first_across_the_scrollback() {
    // Ten lines in a four-row screen: most of them are scrollback.
    let text: String = (0..10).map(|i| format!("row {i} cat\r\n")).collect();
    let t = term(20, 4, &text);
    let m = t.find_all("cat");
    assert_eq!(m.len(), 10);
    assert!(m.windows(2).all(|w| w[0].line > w[1].line), "newest first");
    assert!(
        m.last().is_some_and(|s| s.line < 0),
        "the oldest is scrollback"
    );
    assert_eq!((m[0].col, m[0].end_col), (6, 8));
}

#[test]
fn smart_case_folds_unless_the_query_has_a_capital() {
    let t = term(30, 4, "Cat cat CAT\r\n");
    assert_eq!(t.find_all("cat").len(), 3);
    assert_eq!(t.find_all("Cat").len(), 1);
    assert_eq!(t.find_all("CAT").len(), 1);
}

#[test]
fn punctuation_is_literal_not_regex() {
    let t = term(30, 4, "a.b a+b axb\r\n");
    assert_eq!(t.find_all("a.b").len(), 1);
    assert_eq!(t.find_all("a+b").len(), 1);
    assert_eq!(t.find_all("(").len(), 0);
    let t = term(30, 4, "let v: Vec<u8> = a < b > c; \\d \\w\r\n");
    assert_eq!(
        t.find_all("Vec<u8>").len(),
        1,
        "< and > are not word boundaries"
    );
    assert_eq!(t.find_all("<").len(), 2);
    assert_eq!(t.find_all(">").len(), 2);
    assert_eq!(
        t.find_all("\\d").len(),
        1,
        "a typed backslash is a backslash"
    );
    assert_eq!(t.find_all("\\w").len(), 1);
    assert!(t.find_all("").is_empty());
}

#[test]
fn a_word_a_soft_wrap_split_is_one_match_over_two_lines() {
    let t = term(10, 4, "123456needle");
    let m = t.find_all("needle");
    assert_eq!(m.len(), 1);
    assert_eq!((m[0].line, m[0].end_line), (0, 1));
}

#[test]
fn a_flood_keeps_the_newest_cap() {
    let text = "a".repeat(6000);
    let t = term(100, 4, &text);
    let m: Vec<FindSpan> = t.find_all("a");
    assert_eq!(m.len(), crate::FIND_CAP);
}

#[test]
fn a_no_match_search_over_a_full_scrollback_is_cheap() {
    let mut t = term(100, 24, "");
    let text: String = (0..10_000)
        .map(|i| format!("line {i} of filler text\r\n"))
        .collect();
    t.feed(text.as_bytes());
    let start = std::time::Instant::now();
    assert!(t.find_all("zzqx").is_empty());
    eprintln!("no-match search over 10k lines: {:?}", start.elapsed());
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
}
