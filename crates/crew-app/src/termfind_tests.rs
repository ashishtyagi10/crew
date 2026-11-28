use super::*;
use crew_term::{GridSize, HeadlessTerm, TermModel};

fn found(text: &str, query: &str) -> (TermFind, HeadlessTerm) {
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
    t.feed(text.as_bytes());
    let mut f = TermFind {
        query: query.into(),
        ..Default::default()
    };
    f.set_matches(t.find_all(query), t.screen_line());
    (f, t)
}

fn ten_cats() -> String {
    (0..10).map(|i| format!("row {i} cat\r\n")).collect()
}

#[test]
fn counts_every_match_across_the_scrollback_newest_first() {
    let (f, t) = found(&ten_cats(), "cat");
    assert_eq!(f.matches.len(), 10);
    assert_eq!(f.count(), Some(("1/10".into(), false)));
    assert!(f.matches.windows(2).all(|w| w[0].line > w[1].line));
    assert!(t.history_lines() > 0, "most of them are scrollback");
    assert!(f.matches[9].line < 0);
}

#[test]
fn stepping_older_then_newer_wraps_both_ways() {
    let (mut f, t) = found(&ten_cats(), "cat");
    let top = t.screen_line();
    f.step(true, top);
    assert_eq!(f.sel, 1, "Enter goes to the next match back");
    f.step(false, top);
    f.step(false, top);
    assert_eq!(f.sel, 9, "newer than the newest wraps to the oldest");
    f.step(true, top);
    assert_eq!(f.sel, 0, "older than the oldest wraps to the newest");
    assert_eq!(f.count(), Some(("1/10".into(), false)));
}

#[test]
fn smart_case_and_the_empty_and_missing_labels() {
    let (f, _) = found("Cat cat CAT\r\n", "cat");
    assert_eq!(f.matches.len(), 3);
    let (f, _) = found("Cat cat CAT\r\n", "Cat");
    assert_eq!(f.matches.len(), 1, "a capital makes it case-sensitive");
    let (f, _) = found("Cat\r\n", "zzz");
    assert_eq!(f.count(), Some(("no matches".into(), true)));
    let (f, _) = found("Cat\r\n", "");
    assert_eq!(f.count(), None);
}

#[test]
fn new_output_does_not_slide_the_selection_to_another_match() {
    let (mut f, mut t) = found(&ten_cats(), "cat");
    f.step(true, t.screen_line());
    f.step(true, t.screen_line());
    let before = f.matches[f.sel];
    let abs = |s: &crew_term::FindSpan, top: u64| i64::from(s.line) + top as i64;
    let was = abs(&before, t.screen_line());
    t.feed(b"more output\r\nand another cat\r\n");
    f.set_matches(t.find_all("cat"), t.screen_line());
    assert_eq!(f.matches.len(), 11);
    assert_eq!(abs(&f.matches[f.sel], t.screen_line()), was);
    assert_eq!(f.sel, 3, "one newer match now sits ahead of it");
}

#[test]
fn editing_restarts_at_the_newest_match() {
    let (mut f, t) = found(&ten_cats(), "cat");
    f.step(true, t.screen_line());
    f.edit(Some('x'));
    assert_eq!((f.query.as_str(), f.sel), ("catx", 0));
    f.edit(None);
    f.edit(None);
    assert_eq!(f.query, "ca");
}

#[test]
fn tints_flag_the_current_split_a_wrap_and_skip_the_bar_row() {
    let (f, _) = found("\r\n123456789012345needle\r\ny\r\ny\r\ny\r\n", "needle");
    // The match sits on lines -2 and -1: scrolled back 3, rows 1 and 2.
    let runs = f.tints(3, 20, 4);
    assert_eq!(runs.len(), 2, "one run per row the wrapped word touches");
    assert!(runs.iter().all(|r| r.current));
    assert_eq!((runs[0].row, runs[0].from, runs[0].to), (1, 15, 19));
    assert_eq!((runs[1].row, runs[1].from, runs[1].to), (2, 0, 0));
    assert_eq!(f.bar_row(3, 4), 0);
    // One line less back and it reaches row 0: the bar steps to the bottom.
    assert_eq!(f.bar_row(2, 4), 3);
    assert_eq!(f.tints(2, 20, 4).len(), 2, "both rows still tinted");
    assert!(f.tints(0, 20, 4).is_empty(), "scrolled out of view");
}

#[test]
fn reveal_scrolls_only_when_the_match_is_out_of_view() {
    let (mut f, t) = found(&ten_cats(), "cat");
    let (rows, hist) = (4, t.history_lines());
    assert_eq!(f.reveal(0, rows, hist), None, "the newest is on screen");
    f.sel = 9;
    let to = f.reveal(0, rows, hist).expect("the oldest is not");
    let row = i64::from(f.matches[9].line) + to as i64;
    assert!((0..i64::from(rows)).contains(&row), "lands in view");
    assert_eq!(f.bar_row(to, rows), if row == 0 { rows - 1 } else { 0 });
    assert_eq!(f.reveal(to, rows, hist), None, "and then stays put");
}
