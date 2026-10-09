//! Where the chat's wrap breaks a row.
use super::wrap_indices;

fn rows(s: &str, cols: usize) -> Vec<String> {
    let full: Vec<char> = s.chars().collect();
    wrap_indices(&full, cols)
        .into_iter()
        .map(|(a, b)| full[a..b].iter().collect())
        .collect()
}

/// A list's separator ends the row, as a comma does — the next row never
/// opens on it (`· 12s`), which is the prose wrap's rule too. From a width
/// that holds the longest item and its dot: narrower, a lone word fills the
/// row and there is no other place to break.
#[test]
fn no_row_opens_on_a_separator() {
    let s = "3 done \u{b7} 1 failed \u{b7} 12s total \u{b7} telegram:8812 \u{b7} slack:9000";
    for cols in 16..40 {
        for r in rows(s, cols).iter().skip(1) {
            assert!(
                !r.starts_with('\u{b7}'),
                "{cols}: a row opens on the dot: {r:?}"
            );
        }
    }
    let s = "the plan is ready \u{2014} or draft another";
    for cols in 8..30 {
        for r in rows(s, cols).iter().skip(1) {
            assert!(
                !r.starts_with('\u{2014}'),
                "{cols}: a row opens on the dash: {r:?}"
            );
        }
    }
}

/// Every row still fits: the moved break never overruns the width.
#[test]
fn a_moved_break_still_fits() {
    let s = "a \u{b7} b \u{b7} c \u{b7} d \u{b7} e \u{b7} f";
    for cols in 3..14 {
        for r in rows(s, cols) {
            assert!(r.chars().count() <= cols, "{cols}: {r:?}");
        }
    }
}

/// A word that ends exactly at the edge breaks at the space after it. The
/// break was only ever looked for INSIDE the row, so `bravo` at five
/// columns was cut into `brav` / ` o`, and a row whose last word just fit
/// handed that word to the next row anyway.
#[test]
fn a_word_that_fills_the_row_breaks_at_the_space_after_it() {
    assert_eq!(rows("alpha bravo eight", 5), ["alpha", "bravo", "eight"]);
    assert_eq!(rows("one two three four", 7), ["one two", "three", "four"]);
}

fn pretty(s: &str, cols: usize) -> Vec<String> {
    let c: Vec<char> = s.chars().collect();
    let r = super::wrap_pretty(&c, cols);
    r.iter().map(|&(a, b)| c[a..b].iter().collect()).collect()
}

/// `/keys` ended descriptions on `bar` alone. The row above gives it a word
/// when the two still fit; when they do not, or there is no word to give,
/// or the last row is a hard-broken word's tail, the plain wrap stands.
#[test]
fn the_last_row_is_not_one_word_when_the_row_above_can_spare_one() {
    let s = "toggle the left nav bar";
    assert_eq!(rows(s, 19), ["toggle the left nav", "bar"]);
    assert_eq!(pretty(s, 19), ["toggle the left", "nav bar"]);
    assert_eq!(
        pretty("one two three four", 7),
        ["one two", "three", "four"]
    );
    assert_eq!(pretty("ab cdefghij", 4), rows("ab cdefghij", 4));
    assert_eq!(pretty("a \u{b7} b", 3), rows("a \u{b7} b", 3));
    for cols in 3..30 {
        for r in pretty("drag a pane by its title bar to swap it", cols) {
            assert!(r.chars().count() <= cols, "{cols}: {r:?}");
        }
    }
}
