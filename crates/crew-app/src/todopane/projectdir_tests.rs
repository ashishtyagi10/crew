use std::collections::BTreeMap;
use std::path::PathBuf;

use super::{candidates, git_root, load_at, resolve, save_at};

/// A throwaway tree: `<tmp>/code/{crew,hive}` with `crew` a checkout, and
/// `<tmp>/code/hive/deep` a directory inside the other project.
fn tree() -> (tempfile::TempDir, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let code = t.path().join("code");
    std::fs::create_dir_all(code.join("crew").join(".git")).unwrap();
    std::fs::create_dir_all(code.join("hive").join("deep")).unwrap();
    (t, code)
}

#[test]
fn a_binding_wins_over_every_inference() {
    let (_t, code) = tree();
    let mut bound = BTreeMap::new();
    bound.insert("crew".to_string(), code.join("hive"));
    let dirs = vec![code.join("crew")];
    assert_eq!(
        resolve("Crew", &bound, &dirs),
        Some(code.join("hive")),
        "the user said where it is, case aside"
    );
}

#[test]
fn a_pane_in_a_directory_of_that_name_places_the_project() {
    let (_t, code) = tree();
    let dirs = vec![code.join("hive").join("deep"), code.join("crew")];
    assert_eq!(
        resolve("crew", &BTreeMap::new(), &dirs),
        Some(code.join("crew"))
    );
}

#[test]
fn a_sibling_of_a_pane_directory_places_the_project() {
    let (_t, code) = tree();
    // Only a pane inside hive: crew is found as hive's sibling.
    let dirs = vec![code.join("hive")];
    assert_eq!(
        resolve("crew", &BTreeMap::new(), &dirs),
        Some(code.join("crew"))
    );
    assert_eq!(
        resolve("nowhere", &BTreeMap::new(), &dirs),
        None,
        "never a guess: an unknown name resolves to nothing"
    );
}

#[test]
fn a_pane_deep_in_a_checkout_places_it_by_its_git_root() {
    let (_t, code) = tree();
    let inner = code.join("crew").join("crates").join("x");
    std::fs::create_dir_all(&inner).unwrap();
    assert_eq!(git_root(&inner), Some(code.join("crew")));
    assert_eq!(git_root(&code.join("hive")), None);
    let c = candidates("crew", std::slice::from_ref(&inner));
    assert_eq!(c, vec![code.join("crew")], "the root once, not per rung");
}

#[test]
fn candidates_never_repeat_and_skip_files() {
    let (_t, code) = tree();
    std::fs::write(code.join("notes"), "x").unwrap();
    let dirs = vec![code.join("crew"), code.join("hive"), code.join("crew")];
    assert_eq!(candidates("crew", &dirs), vec![code.join("crew")]);
    assert!(
        candidates("notes", &dirs).is_empty(),
        "a file is not a project"
    );
}

#[test]
fn the_registry_round_trips_and_a_missing_file_is_empty() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("nested").join("projects.toml");
    assert!(load_at(Some(&p)).is_empty());
    let mut m = BTreeMap::new();
    m.insert("crew".to_string(), PathBuf::from("/tmp/crew"));
    save_at(Some(&p), &m);
    assert_eq!(load_at(Some(&p)), m);
    assert!(load_at(None).is_empty());
}
