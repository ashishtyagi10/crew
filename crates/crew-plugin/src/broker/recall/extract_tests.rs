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
