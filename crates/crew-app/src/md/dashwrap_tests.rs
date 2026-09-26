//! A reply's prose never opens a row on a spaced dash.
use crate::md::render_chat;

const REPLY: &str = "Suggested first pick: item 1. It finishes a goal you already \
    started \u{2014} touches only focus logic and one glance card \u{2014} and is \
    easily tested with the existing TailWatch tests.";

fn rows(text: &str, cols: usize) -> Vec<String> {
    render_chat(text, cols)
        .iter()
        .map(|l| l.spans.iter().map(|s| s.text.as_str()).collect())
        .collect()
}

#[test]
fn no_row_opens_on_a_dash() {
    for cols in 12..=120 {
        for row in rows(REPLY, cols) {
            assert!(!row.trim_start().starts_with('\u{2014}'), "{cols}: {row:?}");
        }
    }
}

/// The words all survive, in order, whatever the width.
#[test]
fn moving_the_break_keeps_every_word() {
    let words: Vec<&str> = REPLY.split_whitespace().collect();
    for cols in [20, 33, 47, 64, 80] {
        let joined = rows(REPLY, cols).join(" ");
        assert_eq!(
            joined.split_whitespace().collect::<Vec<_>>(),
            words,
            "{cols}"
        );
    }
}

/// An unspaced dash (`well—known`) is part of a word, and a dash with no
/// word before it on the row has nowhere to go: both are left alone.
#[test]
fn a_dash_with_no_word_to_join_is_left_alone() {
    let text = "\u{2014} a list-less aside that has to wrap somewhere";
    assert!(rows(text, 16)[0].starts_with('\u{2014}'));
}
