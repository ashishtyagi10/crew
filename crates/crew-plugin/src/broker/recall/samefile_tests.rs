use super::*;

const FULL: &str = "crates/crew-app/src/panelcard_tests.rs";

fn graph(paths: &[&str]) -> Graph {
    let mut g = Graph::default();
    for p in paths {
        g.touch(Kind::File, p, p, 1);
    }
    g
}

#[test]
fn a_bare_name_is_the_one_known_path_that_ends_with_it() {
    let g = graph(&[FULL, "src/nav.rs"]);
    assert_eq!(filed(&g, "panelcard_tests.rs"), FULL);
    assert_eq!(id(&g, "PanelCard_tests.rs"), g.find(Kind::File, FULL));
}

#[test]
fn two_paths_with_the_name_are_two_files_and_nothing_is_guessed() {
    let g = graph(&["src/a/mod.rs", "src/b/mod.rs"]);
    assert_eq!(filed(&g, "mod.rs"), "mod.rs");
    assert_eq!(id(&g, "mod.rs"), None);
}

#[test]
fn a_name_that_only_ends_another_name_is_not_that_file() {
    // `card.rs` is not `src/panelcard.rs`: the match starts at a separator.
    let g = graph(&["src/panelcard.rs"]);
    assert_eq!(filed(&g, "card.rs"), "card.rs");
}

#[test]
fn a_path_is_filed_as_written_and_a_known_bare_name_stays_itself() {
    let g = graph(&[FULL, "Cargo.toml"]);
    assert_eq!(
        filed(&g, "other/panelcard_tests.rs"),
        "other/panelcard_tests.rs"
    );
    assert_eq!(filed(&g, "Cargo.toml"), "Cargo.toml");
}

#[test]
fn a_list_names_a_file_once_when_it_holds_both_spellings() {
    let got = once(vec![
        "panelcard_tests.rs".into(),
        FULL.into(),
        "Cargo.toml".into(),
    ]);
    assert_eq!(got, vec![FULL.to_string(), "Cargo.toml".to_string()]);
}
