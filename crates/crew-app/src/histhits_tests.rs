//! The marks explain the match: a run where it is a substring, the letters
//! where it is a subsequence.
use super::*;

#[test]
fn a_substring_is_marked_as_one_run() {
    assert_eq!(hits("cargo clippy --workspace", "car"), [0, 1, 2]);
    assert_eq!(hits("git CARgo", "car"), [4, 5, 6], "any case");
}

#[test]
fn a_subsequence_marks_the_letters_it_used() {
    let l = "why is the sidebar chart a smear";
    let h = hits(l, "cam");
    let picked: String = h.iter().map(|&i| l.chars().nth(i).unwrap()).collect();
    assert_eq!(picked, "cam");
    assert!(h.windows(2).all(|w| w[0] < w[1]), "in order: {h:?}");
}

#[test]
fn nothing_to_mark() {
    assert!(hits("ls -la", "").is_empty());
    assert!(hits("ls", "lsof").is_empty());
    assert!(hits("git status", "zz").is_empty());
}

/// End to end: the popup's rows carry the marks for the palette to wash.
#[test]
fn the_popup_rows_carry_their_marks() {
    let h = crate::chathistsearch::HistSearch {
        query: "car".into(),
        saved: String::new(),
        matches: vec!["cargo test".into()],
        sel: 0,
    };
    let rows = crate::chathistsearch::items(&h);
    assert_eq!(rows[0].hit, [0, 1, 2]);
}

/// A mid-sentence `@` mention marks its rows too, past the `@` and a skill's
/// `skill:` prefix.
#[test]
fn a_mention_row_marks_its_label_past_the_prefix() {
    use crate::chatmention::MentionEntry;
    let skill = MentionEntry::Skill {
        name: "verify".into(),
        desc: String::new(),
    };
    assert_eq!(
        mention_hits(&skill, "ri"),
        [9, 10],
        "`ri` in `@skill:verify`"
    );
    let file = MentionEntry::File("src/main.rs".into());
    assert_eq!(mention_hits(&file, "main"), [5, 6, 7, 8]);
}
