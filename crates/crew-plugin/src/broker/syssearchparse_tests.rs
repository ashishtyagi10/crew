//! The fixture is DuckDuckGo's real markup, trimmed: two results with the
//! attribute order, the `&amp;rut=` signature and the `<b>` highlighting the
//! live page sends. If the endpoint changes shape, these fail — which is the
//! point of keeping them.
use super::*;

const PAGE: &str = r#"
<div class="results">
  <div class="result results_links">
    <h2 class="result__title">
      <a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fdoc.rust%2Dlang.org%2Freference%2Flifetime%2Delision.html&amp;rut=b5164473af29">Lifetime elision - The Rust Reference</a>
    </h2>
    <a class="result__snippet" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fdoc.rust%2Dlang.org%2Freference%2Flifetime%2Delision.html&amp;rut=b5164473af29"><b>Lifetime</b> <b>elision</b> Rust has rules that allow lifetimes to be elided.</a>
  </div>
  <div class="result results_links">
    <h2 class="result__title">
      <a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fdoc.rust%2Dlang.org%2Fnomicon%2Flifetime%2Delision.html&amp;rut=6a1a37e991">The Rustonomicon</a>
    </h2>
    <a class="result__snippet" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fdoc.rust%2Dlang.org%2Fnomicon%2Flifetime%2Delision.html&amp;rut=6a1a37e991">In order to make common patterns more ergonomic.</a>
  </div>
</div>
"#;

#[test]
fn a_result_is_title_destination_and_snippet() {
    let hits = hits(PAGE, 8);
    assert_eq!(hits.len(), 2, "two results in the fixture");
    assert_eq!(hits[0].title, "Lifetime elision - The Rust Reference");
    assert_eq!(
        hits[0].url,
        "https://doc.rust-lang.org/reference/lifetime-elision.html"
    );
    assert_eq!(
        hits[0].snippet,
        "Lifetime elision Rust has rules that allow lifetimes to be elided."
    );
}

#[test]
fn the_redirector_never_reaches_the_model() {
    // The href is a duckduckgo.com hop carrying a signature. Citing it would
    // name a URL no reader can check, and fetching it would spend a whole
    // round trip arriving where we already knew we were going.
    for hit in hits(PAGE, 8) {
        assert!(
            !hit.url.contains("duckduckgo.com"),
            "still the redirect: {}",
            hit.url
        );
        assert!(!hit.url.contains("rut="), "signature kept: {}", hit.url);
        assert!(hit.url.starts_with("https://"), "no scheme: {}", hit.url);
    }
}

#[test]
fn results_keep_the_order_the_page_ranked_them() {
    let hits = hits(PAGE, 8);
    assert_eq!(hits[1].title, "The Rustonomicon");
    assert!(hits[1].url.ends_with("/nomicon/lifetime-elision.html"));
}

#[test]
fn max_cuts_the_list_from_the_bottom() {
    let hits = hits(PAGE, 1);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Lifetime elision - The Rust Reference");
}

#[test]
fn a_result_without_a_snippet_still_counts() {
    // Dropping it would silently renumber every result below it.
    let page = PAGE.replace("result__snippet", "result__gone");
    let hits = hits(&page, 8);
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].snippet, "");
    assert_eq!(
        hits[0].url,
        "https://doc.rust-lang.org/reference/lifetime-elision.html"
    );
}

#[test]
fn renamed_classes_yield_nothing_rather_than_nonsense() {
    // The day the endpoint changes shape, search must come back empty (and
    // say so) rather than hand the model half-parsed markup.
    assert!(hits(&PAGE.replace("result__a", "result__x"), 8).is_empty());
    assert!(hits("<html><body>no results here</body></html>", 8).is_empty());
}

#[test]
fn the_query_is_encoded_into_the_no_javascript_endpoint() {
    let url = query_url("rust lifetime elision");
    assert_eq!(
        url,
        "https://html.duckduckgo.com/html/?q=rust%20lifetime%20elision"
    );
    assert!(query_url("  spaced  ").ends_with("?q=spaced"), "untrimmed");
    // A query is user text: & and = must not become query syntax.
    let url = query_url("a&b=c");
    assert!(url.ends_with("?q=a%26b%3Dc"), "{url}");
}
