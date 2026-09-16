//! Rendering and the empty query. The network half is not tested here: it is
//! `sysfetch::raw`, which has its own guards and its own tests.
use super::*;

fn hit(title: &str, url: &str, snippet: &str) -> Hit {
    Hit {
        title: title.into(),
        url: url.into(),
        snippet: snippet.into(),
    }
}

#[test]
fn an_empty_query_never_reaches_the_network() {
    for q in ["", "   ", "\n\t"] {
        let err = search(q).unwrap_err();
        assert!(err.starts_with("search: no query"), "{err}");
    }
}

#[test]
fn every_result_is_numbered_with_its_url_on_its_own_line() {
    let out = render(
        "rust elision",
        &[
            hit("Lifetime elision", "https://doc.rust-lang.org/a", "rules"),
            hit("The Rustonomicon", "https://doc.rust-lang.org/b", "more"),
        ],
    );
    assert!(
        out.contains("2 results for \u{201c}rust elision\u{201d}"),
        "{out}"
    );
    assert!(
        out.contains("\n1. Lifetime elision\n   https://doc.rust-lang.org/a\n"),
        "{out}"
    );
    assert!(
        out.contains("\n2. The Rustonomicon\n   https://doc.rust-lang.org/b\n"),
        "{out}"
    );
    // The whole reason the URL is carried: the model is told what to do with it.
    assert!(out.contains("sys:fetch"), "no next step: {out}");
    assert!(out.contains("cite the URL"), "no citation ask: {out}");
}

#[test]
fn one_result_is_not_called_results() {
    let out = render("q", &[hit("T", "https://e.com", "")]);
    assert!(out.starts_with("1 result for"), "{out}");
    assert!(!out.contains("1 results"), "{out}");
}

#[test]
fn nothing_found_says_so_instead_of_an_empty_block() {
    // An empty string reads as a broken tool; the model must be able to tell
    // "the web has nothing" from "the search failed".
    let out = render("wehrijgkn", &[]);
    assert_eq!(out, "no results for \u{201c}wehrijgkn\u{201d}");
}

#[test]
fn a_result_with_no_snippet_prints_no_blank_line() {
    let out = render("q", &[hit("T", "https://e.com", "   ")]);
    assert!(out.ends_with("1. T\n   https://e.com\n"), "{out:?}");
}

#[test]
fn a_long_snippet_is_cut_with_a_marker_and_stays_on_one_line() {
    let long = "word ".repeat(200);
    let out = snippet(&long);
    assert!(
        out.chars().count() <= SNIPPET_CAP + 1,
        "{}",
        out.chars().count()
    );
    assert!(out.ends_with('\u{2026}'), "no marker: {out}");
    assert!(!out.contains('\n'), "wrapped into the margin");
}

#[test]
fn a_snippet_of_several_lines_is_flattened() {
    assert_eq!(snippet("two\nlines   here"), "two lines here");
}
