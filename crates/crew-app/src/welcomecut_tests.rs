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
