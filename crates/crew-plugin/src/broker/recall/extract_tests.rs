use super::*;

#[test]
fn a_path_is_recognised_by_its_slashes_or_its_extension() {
    let got = paths("look at crates/crew-app/src/nav.rs and Cargo.toml please");
    assert_eq!(got, vec!["crates/crew-app/src/nav.rs", "Cargo.toml"]);
}

#[test]
fn prose_and_urls_are_not_filed_as_files() {
    assert!(paths("see https://example.com/docs for the rest").is_empty());
    assert!(paths("the swarm decides which tools to call").is_empty());
}

#[test]
fn a_path_is_kept_once_however_often_it_is_named() {
    let got = paths("src/a.rs then src/a.rs again, and (src/a.rs).");
    assert_eq!(got, vec!["src/a.rs"]);
}

#[test]
fn the_topics_are_the_words_the_turn_kept_using() {
    let t = topics("the swarm planner asked the swarm to replan the swarm graph");
    assert_eq!(t.first().map(String::as_str), Some("swarm"));
    assert!(t.contains(&"planner".to_string()));
}

#[test]
fn filler_short_words_and_bare_numbers_are_not_topics() {
    let t = topics("this should just be about 2026 and the very first one");
    assert!(t.is_empty(), "{t:?} came back as topics");
}

#[test]
fn one_chatty_turn_cannot_fill_the_graph_with_its_own_vocabulary() {
    let text: String = (0..50).map(|i| format!("word{i} word{i} ")).collect();
    assert_eq!(topics(&text).len(), TOPICS_MAX);
}

#[test]
fn a_cited_url_is_extracted_and_prose_punctuation_is_not_part_of_it() {
    let got = urls("see https://doc.rust-lang.org/std/, which says so.");
    assert_eq!(got, vec!["https://doc.rust-lang.org/std/"]);
    // The trailing slash IS the address; the comma is the sentence.
    let got = urls("(https://example.com/a).");
    assert_eq!(got, vec!["https://example.com/a"]);
    let got = urls("quoted \u{201c}https://example.com/b\u{201d} here");
    assert_eq!(got, vec!["https://example.com/b"]);
}

#[test]
fn only_http_urls_count_and_each_only_once() {
    let text = "ftp://nope.com/x mailto:a@b.c https://a.example/p https://a.example/p";
    assert_eq!(urls(text), vec!["https://a.example/p"]);
}

#[test]
fn a_turn_that_pasted_a_whole_result_list_is_capped() {
    let text = (0..20)
        .map(|i| format!("https://example.com/page{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(urls(&text).len(), URLS_MAX);
}

#[test]
fn a_scheme_with_nothing_after_it_is_not_a_page() {
    assert!(urls("https:// http:// https://a").is_empty());
    assert!(urls(&format!("https://e.com/{}", "x".repeat(400))).is_empty());
}

#[test]
fn a_url_is_not_filed_as_a_path_and_a_path_is_not_filed_as_a_page() {
    // The two extractors partition the tokens between them; neither claims
    // the other's, or one turn would remember the same thing twice.
    let text = "crates/crew-plugin/src/lib.rs and https://example.com/a/b.rs";
    assert_eq!(paths(text), vec!["crates/crew-plugin/src/lib.rs"]);
    assert_eq!(urls(text), vec!["https://example.com/a/b.rs"]);
}
