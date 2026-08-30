//! What the labels have to be: on the right things, cheapest where the eye
//! already is, and never ambiguous about when a press is finished.
use super::*;

fn rows(lines: &[&str]) -> Vec<Vec<char>> {
    lines.iter().map(|l| l.chars().collect()).collect()
}

fn scan(lines: &[&str]) -> Hints {
    Hints::scan(&rows(lines)).expect("something to label")
}

#[test]
fn urls_paths_and_hashes_are_what_gets_labelled() {
    let h = scan(&[
        "listening on https://localhost:8080/status",
        "error in src/main.rs:42 — see docs/CREW.md",
        "commit 9f3ab1c7 fixed it",
    ]);
    let kinds: Vec<Kind> = h.targets.iter().map(|t| t.kind).collect();
    let texts: Vec<&str> = h.targets.iter().map(|t| t.text.as_str()).collect();
    assert!(texts.contains(&"https://localhost:8080/status"));
    assert!(texts.contains(&"src/main.rs:42"));
    assert!(texts.contains(&"docs/CREW.md"));
    assert!(texts.contains(&"9f3ab1c7"), "a commit hash: {texts:?}");
    assert!(kinds.contains(&Kind::Url) && kinds.contains(&Kind::Path));
}

/// A URL is also, textually, a path with slashes in it. It must be labelled
/// once, as the URL — two labels on one span would put a tag in the middle of
/// the thing it is pointing at.
#[test]
fn a_url_is_labelled_once_not_twice() {
    let h = scan(&["see https://example.invalid/a/b.md for more"]);
    assert_eq!(h.targets.len(), 1, "{:?}", h.targets);
    assert_eq!(h.targets[0].kind, Kind::Url);
}

/// The newest output is at the bottom and is what you almost always want, so
/// it gets the first letter of the alphabet, not the last.
#[test]
fn the_cheapest_label_goes_to_the_newest_line() {
    let h = scan(&["old.txt", "newer.txt", "newest.txt"]);
    let first = h.targets.iter().find(|t| t.label == "a").expect("an `a`");
    assert_eq!(first.text, "newest.txt");
}

/// A number is not a hash. Labelling every port, byte count and line number
/// buries the two things on screen that are actually object ids.
#[test]
fn long_runs_of_digits_are_not_hashes() {
    assert!(Hints::scan(&rows(&["compiled 1274839 lines in 2500 ms"])).is_none());
    let h = scan(&["blob 4f2b9ac written"]);
    assert_eq!(h.targets.len(), 1);
    assert_eq!(h.targets[0].text, "4f2b9ac");
}

/// A hex-looking piece of a longer word is not an object id either.
#[test]
fn a_hash_has_to_be_a_word_of_its_own() {
    assert!(Hints::scan(&rows(&["decafbadness"])).is_none());
    assert!(Hints::scan(&rows(&["xdeadbeefx"])).is_none());
}

#[test]
fn a_screen_with_nothing_to_reach_opens_no_mode() {
    assert!(
        Hints::scan(&rows(&["all done", "nothing here"])).is_none(),
        "a mode with no targets eats the next key you press"
    );
}

#[test]
fn pressing_a_label_picks_it_and_a_capital_means_open() {
    let mut h = scan(&["one.txt two.txt"]);
    let want = h.targets[0].clone();
    let label = want.label.chars().next().unwrap();
    assert_eq!(
        h.clone().press(label),
        Press::Pick(Box::new(want.clone()), false)
    );
    assert_eq!(
        h.press(label.to_ascii_uppercase()),
        Press::Pick(Box::new(want), true),
        "a capital opens rather than copies"
    );
}

/// A key that matches nothing ends the mode rather than sitting there
/// swallowing keys the pane wanted.
#[test]
fn a_press_that_matches_nothing_is_a_miss() {
    let mut h = scan(&["one.txt"]);
    assert_eq!(h.press('§'), Press::Miss);
}

/// With more targets than letters every label is a pair — a single letter
/// that is also the start of a pair could never be finished.
#[test]
fn no_label_is_a_prefix_of_another() {
    for n in [1, 5, 26, 27, 60, 300] {
        let ls = labels_for(n);
        assert_eq!(ls.len(), n);
        for (i, a) in ls.iter().enumerate() {
            for (j, b) in ls.iter().enumerate() {
                assert!(
                    i == j || !b.starts_with(a.as_str()),
                    "{n}: {a:?} is a prefix of {b:?}"
                );
            }
        }
    }
}

/// Two presses for a two-letter label, and the first one only narrows.
#[test]
fn a_two_letter_label_takes_two_presses() {
    let many: Vec<String> = (0..40).map(|i| format!("f{i}.txt")).collect();
    let refs: Vec<&str> = many.iter().map(String::as_str).collect();
    let mut h = scan(&refs);
    let label: Vec<char> = h.targets[0].label.chars().collect();
    assert_eq!(label.len(), 2, "40 targets need pairs");
    assert_eq!(h.press(label[0]), Press::Pending);
    assert!(matches!(h.press(label[1]), Press::Pick(..)));
}

/// The labels have to be legible over whatever they land on, and they must
/// not move the text they cover.
#[test]
fn the_labels_are_drawn_over_the_first_cells_of_their_target() {
    let h = scan(&["see docs/CREW.md now"]);
    let mut cells: Vec<CellView> = (0..20u16)
        .map(|col| CellView {
            col,
            row: 0,
            c: 'x',
            ..Default::default()
        })
        .collect();
    let before = cells.len();
    h.mark(&mut cells);
    assert_eq!(
        cells.len(),
        before,
        "a label replaces a cell, never adds one"
    );
    let at = cells
        .iter()
        .find(|c| c.col == h.targets[0].col && c.row == 0)
        .expect("a cell where the target starts");
    assert_eq!(at.c, h.targets[0].label.chars().next().unwrap());
    assert_ne!(
        at.bg,
        CellView::default().bg,
        "the tag carries its own field"
    );
}
