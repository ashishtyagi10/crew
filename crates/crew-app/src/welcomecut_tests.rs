//! The "new in" line is cut between words, at every width.
use super::*;

const HEAD: &str = "Every picker shows what you typed, marked where it matched";

#[test]
fn the_headline_is_cut_between_words() {
    let words: Vec<&str> = HEAD.split(' ').map(|w| w.trim_end_matches(',')).collect();
    for room in 12..HEAD.len() {
        let fit = fit_head(HEAD, room);
        assert!(fit.chars().count() <= room, "{room}: {fit:?}");
        let kept = fit.strip_suffix('\u{2026}').expect("cut");
        let last = kept.split(' ').next_back().unwrap();
        assert!(
            words.contains(&last),
            "{room}: `{last}` is half a word — {fit:?}"
        );
    }
    assert_eq!(
        fit_head(HEAD, HEAD.len()),
        HEAD,
        "a headline that fits is whole"
    );
}

// The "new in" line speaks in the welcome screen's lowercase.

#[test]
fn a_plain_opening_word_is_lowered() {
    assert_eq!(
        super::lowered("The palette shows three more shortcuts"),
        "the palette shows three more shortcuts"
    );
    assert_eq!(
        super::lowered("A swarm's task titles line up"),
        "a swarm's task titles line up"
    );
    assert_eq!(
        super::lowered("Failures are worded the same way"),
        "failures are worded the same way"
    );
}

#[test]
fn initialisms_commands_and_names_keep_their_capitals() {
    for head in [
        "AI is written in capitals everywhere",
        "README names the commands",
        "`/far` shows one caret",
        "\"AI\" is written in capitals",
        "Far's F8 says moved",
        "Claude Code relays live",
        "agent smith stops pointing at retired commands",
    ] {
        assert_eq!(super::lowered(head), head);
    }
}

#[test]
fn the_welcome_line_is_lowercase_after_its_lead() {
    let line = super::whats_new(200).expect("room for the news");
    let head = line.split(" \u{b7} ").nth(1).unwrap();
    let first = head.split(|c: char| !c.is_alphanumeric()).next().unwrap();
    let plain = first.chars().skip(1).all(|c| c.is_ascii_lowercase());
    assert!(
        !(plain
            && first.starts_with(|c: char| c.is_ascii_uppercase())
            && !super::PROPER.contains(&first)),
        "{line}"
    );
}
