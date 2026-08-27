use super::*;

/// A per-test fixture: a couple of files, a folder, and a hidden entry.
fn fixture(name: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("crew_pathmenu_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("alpha")).unwrap();
    std::fs::create_dir_all(base.join("beta")).unwrap();
    std::fs::write(base.join("readme.md"), b"x").unwrap();
    std::fs::write(base.join("agenda.txt"), b"x").unwrap();
    std::fs::write(base.join(".secret"), b"x").unwrap();
    base
}

fn labels(text: &str, base: &std::path::Path) -> Vec<String> {
    rows(text, base)
        .expect("a path command")
        .into_iter()
        .map(|i| i.label)
        .collect()
}

#[test]
fn only_path_commands_get_a_listing() {
    let base = fixture("only");
    for cmd in crate::pathcomplete::PATH_COMMANDS {
        assert!(rows(&format!("{cmd} "), &base).is_some(), "{cmd}");
    }
    // A closed-set command keeps its own picker; a freeform one keeps none.
    assert!(rows("/theme ", &base).is_none());
    assert!(rows("/run ", &base).is_none());
    // The space is part of the prefix — `/viewer` is not `/view`.
    assert!(rows("/viewer ", &base).is_none());
    assert!(rows("/view", &base).is_none(), "no argument yet");
}

/// Folders first, then by name: you are usually navigating before you are
/// choosing, and a folder buried among its own files is one you scroll past.
#[test]
fn folders_come_first_then_names() {
    let base = fixture("order");
    assert_eq!(
        labels("/view ", &base),
        vec!["alpha/", "beta/", "agenda.txt", "readme.md"]
    );
}

#[test]
fn the_partial_filters_the_listing_case_insensitively() {
    let base = fixture("filter");
    assert_eq!(labels("/view a", &base), vec!["alpha/", "agenda.txt"]);
    assert_eq!(labels("/view READ", &base), vec!["readme.md"]);
    assert!(labels("/view zz", &base).is_empty());
}

/// Hidden entries appear only once the partial says so — the rule every
/// shell's completion follows, and the reason a home directory is readable.
#[test]
fn hidden_entries_wait_to_be_asked_for() {
    let base = fixture("hidden");
    assert!(!labels("/view ", &base).contains(&".secret".to_string()));
    assert_eq!(labels("/view .", &base), vec![".secret"]);
}

/// A folder is a step, not an answer: accepting it fills `<cmd> dir/` and
/// leaves the bar open, so the next read lists what is inside it.
#[test]
fn a_folder_fills_and_stays_open_while_a_file_runs() {
    let base = fixture("submit");
    let got = rows("/view a", &base).expect("rows");
    let dir = got
        .iter()
        .find(|i| i.label == "alpha/")
        .expect("the folder");
    assert!(!dir.submit, "a folder must not run");
    assert_eq!(dir.fill, "/view alpha/");
    assert_eq!(dir.desc, "folder");
    let file = got.iter().find(|i| i.label == "agenda.txt").expect("file");
    assert!(file.submit, "a file is the answer");
    assert_eq!(file.fill, "/view agenda.txt");
}

/// Walking in: a partial that already names a directory lists what is inside
/// it, and every row carries the whole path so accepting one is complete.
#[test]
fn walking_into_a_folder_lists_it_with_full_paths() {
    let base = fixture("walk");
    std::fs::write(base.join("alpha").join("inner.rs"), b"x").unwrap();
    assert_eq!(labels("/view alpha/", &base), vec!["alpha/inner.rs"]);
    let got = rows("/view alpha/i", &base).expect("rows");
    assert_eq!(got[0].fill, "/view alpha/inner.rs");
}

#[test]
fn a_directory_that_is_not_there_lists_nothing_rather_than_failing() {
    let base = fixture("missing");
    assert!(rows("/view nowhere/x", &base).is_none());
}
