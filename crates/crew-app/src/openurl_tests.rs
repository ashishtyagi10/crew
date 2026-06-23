use super::{find_urls, url_at, url_spans};

#[test]
fn url_spans_reports_char_ranges() {
    let line = "a https://x.io/p b http://y.io c";
    let chars: Vec<char> = line.chars().collect();
    let spans = url_spans(&chars);
    assert_eq!(spans.len(), 2);
    // each span slices back to the exact URL text.
    let s0: String = chars[spans[0].0..spans[0].1].iter().collect();
    let s1: String = chars[spans[1].0..spans[1].1].iter().collect();
    assert_eq!(s0, "https://x.io/p");
    assert_eq!(s1, "http://y.io");
    // a bare scheme produces no span.
    assert!(url_spans(&"see https:// x".chars().collect::<Vec<_>>()).is_empty());
}

#[test]
fn url_at_returns_link_spanning_the_column() {
    let line = "open https://example.com/path now";
    let start = line.find("https").unwrap();
    // a column inside the URL resolves to it...
    assert_eq!(
        url_at(line, start + 5).as_deref(),
        Some("https://example.com/path")
    );
    // ...the first and last URL chars are inside...
    assert_eq!(
        url_at(line, start).as_deref(),
        Some("https://example.com/path")
    );
    // ...but a column in the surrounding words is not a link.
    assert_eq!(url_at(line, 0), None);
    assert_eq!(url_at(line, line.len() - 1), None);
}

#[test]
fn url_at_ignores_bare_scheme_and_trailing_punctuation() {
    // a scheme with no host isn't a link.
    assert_eq!(url_at("see https:// nope", 6), None);
    // trailing ")" is trimmed, so clicking it returns nothing.
    let line = "(https://a.io)";
    assert_eq!(url_at(line, 1).as_deref(), Some("https://a.io"));
    assert_eq!(url_at(line, line.len() - 1), None);
}

#[test]
fn finds_urls_in_order_and_trims_trailers() {
    let text = "see https://example.com/a, and http://b.org/x).\nlast https://z.io";
    let urls = find_urls(text);
    assert_eq!(
        urls,
        vec![
            "https://example.com/a".to_string(),
            "http://b.org/x".to_string(),
            "https://z.io".to_string(),
        ]
    );
    // `/open` with no arg uses the most recent (last) one.
    assert_eq!(find_urls(text).pop().unwrap(), "https://z.io");
}

#[test]
fn ignores_non_urls_and_bare_schemes() {
    // a stray "http" word and a scheme with no host are not URLs.
    assert!(find_urls("nothing http here, https:// either").is_empty());
    assert!(find_urls("plain text, no links").is_empty());
}
