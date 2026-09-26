//! A clamped line gives up whole words to make room for ` … +N`.
use super::*;

fn line(s: &str) -> CardLine {
    s.chars().map(|c| plain(c, (0, 0, 0), false)).collect()
}

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

/// The folded fan reply: the first line filled its row, so the suffix has to
/// take cells from it — and it used to take them from the middle of
/// `touches`.
#[test]
fn a_full_line_gives_up_whole_words() {
    let s = " Suggested first pick: item 1. It finishes a goal you already started, touches";
    let cols = s.chars().count();
    let mut l = line(s);
    append_hidden_suffix(&mut l, 9, cols);
    let t = text(&l);
    assert_eq!(
        t,
        " Suggested first pick: item 1. It finishes a goal you already started, \u{2026} +9"
    );
    assert!(t.chars().count() <= cols, "{t:?}");
}

/// A cut that lands on a space already ends a word: nothing more goes.
#[test]
fn a_cut_on_a_word_end_keeps_that_word() {
    let mut l = line(" alpha beta gamma");
    append_hidden_suffix(&mut l, 2, 16);
    assert_eq!(text(&l), " alpha beta \u{2026} +2");
}

/// One long word has no boundary to back up to: it is cut at a letter, and
/// still fits.
#[test]
fn one_long_word_is_cut_at_a_letter() {
    let mut l = line(" /Users/me/code/crew/crates/crew-app/src/chathidden.rs");
    append_hidden_suffix(&mut l, 3, 30);
    let t = text(&l);
    assert!(t.starts_with(" /Users/me/code/crew/c"), "{t:?}");
    assert!(t.ends_with(" \u{2026} +3"), "{t:?}");
    assert_eq!(t.chars().count(), 30, "{t:?}");
}

/// A line with room to spare is left alone.
#[test]
fn a_short_line_only_gains_the_suffix() {
    let mut l = line(" one");
    append_hidden_suffix(&mut l, 2, 40);
    assert_eq!(text(&l), " one \u{2026} +2");
}
