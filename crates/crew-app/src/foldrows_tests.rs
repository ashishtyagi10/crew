use super::*;

/// `/th` reaches for `/theme`, which lives under `/look` now: the palette
/// offers `/look theme`, inserting it so the theme list opens next.
#[test]
fn an_old_spelling_is_offered_its_new_home() {
    let got = rows("/th");
    assert_eq!(
        got.len(),
        1,
        "{:?}",
        got.iter().map(|r| &r.label).collect::<Vec<_>>()
    );
    assert_eq!(got[0].label, "/look theme");
    assert_eq!(got[0].fill, "/look theme ");
    assert!(!got[0].submit, "a two-step subject opens its own list");
    assert_eq!(got[0].hit, vec![6, 7], "the `th` of theme");
    let clear = rows("/clearall");
    assert_eq!(clear[0].label, "/clear all");
    assert_eq!(clear[0].hit, vec![1, 2, 3, 4, 5, 7, 8, 9]);
    assert_eq!(clear[0].fill, "/clear all");
    assert!(clear[0].submit, "a one-step subject runs");
}

/// Nothing while the query is one letter, still the verb's own name, or
/// has moved on to an argument.
#[test]
fn only_a_real_reach_for_an_old_spelling() {
    assert!(rows("/t").is_empty());
    assert!(rows("/cle").is_empty(), "the /clear row is already there");
    assert!(rows("/theme dark").is_empty());
    assert!(rows("theme").is_empty());
}

/// In the palette: after the commands that begin with the query, before
/// the ones that only hold its letters.
#[test]
fn the_palette_lists_it_after_the_prefix_band() {
    let labels: Vec<String> = crate::suggest::menu_items("/th")
        .into_iter()
        .map(|r| r.label)
        .collect();
    let at = labels
        .iter()
        .position(|l| l == "/look theme")
        .expect("offered");
    assert!(
        labels[..at].iter().all(|l| l.starts_with("/th")),
        "{labels:?}"
    );
}
