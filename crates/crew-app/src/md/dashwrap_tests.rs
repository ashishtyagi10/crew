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

/// A list's separator dot never opens a row either: the row ends on it, the
/// way a comma sits (`1 cancelled ·` over `1s`), or on the last dot that
/// fits, so no part is split — and every word survives.
#[test]
fn no_row_opens_on_a_separator_dot() {
    let text = "swarm \u{b7} 5 tasks \u{b7} 3 done \u{b7} 1 failed \u{b7} 1 cancelled \u{b7} 1s";
    let words: Vec<&str> = text.split_whitespace().collect();
    // From 11: `cancelled ·`, the longest part with its dot, has to fit.
    for cols in 11..=60 {
        let got = rows(text, cols);
        for row in &got {
            assert!(!row.trim_start().starts_with('\u{b7}'), "{cols}: {got:?}");
            assert!(row.chars().count() <= cols, "{cols}: {row:?} overruns");
        }
        let joined = got.join(" ");
        assert_eq!(
            joined.split_whitespace().collect::<Vec<_>>(),
            words,
            "{cols}"
        );
    }
    assert_eq!(
        rows(text, 51),
        [
            "swarm \u{b7} 5 tasks \u{b7} 3 done \u{b7} 1 failed \u{b7} 1 cancelled \u{b7}",
            "1s"
        ]
    );
    assert_eq!(
        rows(text, 50),
        [
            "swarm \u{b7} 5 tasks \u{b7} 3 done \u{b7} 1 failed \u{b7}",
            "1 cancelled \u{b7} 1s"
        ]
    );
}
