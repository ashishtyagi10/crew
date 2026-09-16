use super::*;

/// A small tree: `<tmp>/proj/{crates/,README.md,main.rs}`.
fn tree(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-syspath-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(dir.join("proj").join("crates")).unwrap();
    std::fs::write(dir.join("proj").join("README.md"), "x").unwrap();
    std::fs::write(dir.join("proj").join("main.rs"), "x").unwrap();
    dir
}

#[test]
fn a_misspelled_file_is_answered_with_the_name_that_is_there() {
    let dir = tree("typo");
    let h = hint(dir.join("proj").join("mian.rs").to_str().unwrap()).expect("a hint");
    assert!(h.contains("has no \u{201c}mian.rs\u{201d}"), "{h}");
    assert!(h.contains("did you mean \u{201c}main.rs\u{201d}?"), "{h}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_name_near_nothing_gets_the_directorys_contents_instead() {
    // Nothing to suggest is not nothing to say: what IS there is the fact
    // that narrows the next guess.
    let dir = tree("far");
    let h = hint(dir.join("proj").join("zzzzzz").to_str().unwrap()).expect("a hint");
    assert!(h.contains("it holds"), "{h}");
    assert!(h.contains("main.rs") && h.contains("crates"), "{h}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_wrong_branch_names_where_the_path_stopped_being_real() {
    // Two invented levels: the useful fact is that `proj` has no `src`, not
    // that `proj/src/deep/main.rs` is missing.
    let dir = tree("branch");
    let missing = dir.join("proj").join("src").join("deep").join("main.rs");
    let h = hint(missing.to_str().unwrap()).expect("a hint");
    assert!(h.contains("has no \u{201c}src\u{201d}"), "{h}");
    assert!(h.contains("proj"), "the real parent is not named: {h}");
    assert!(!h.contains("deep"), "blamed the wrong component: {h}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn dotfiles_do_not_fill_the_line() {
    let dir = tree("dots");
    std::fs::create_dir_all(dir.join("proj").join(".git")).unwrap();
    std::fs::write(dir.join("proj").join(".DS_Store"), "x").unwrap();
    let h = hint(dir.join("proj").join("zzzzzz").to_str().unwrap()).expect("a hint");
    assert!(!h.contains(".git") && !h.contains(".DS_Store"), "{h}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_long_directory_is_clipped_with_a_count() {
    let dir = tree("many");
    let many = dir.join("proj").join("crates");
    for i in 0..20 {
        std::fs::write(many.join(format!("f{i:02}.rs")), "x").unwrap();
    }
    let h = hint(many.join("zzzzzz").to_str().unwrap()).expect("a hint");
    assert!(h.contains("\u{2026} +"), "not clipped: {h}");
    assert!(h.len() < 200, "a wall of text: {h}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_empty_directory_or_an_unreadable_one_says_nothing_extra() {
    // A sentence that narrows nothing is worse than the plain error.
    let dir = tree("empty");
    let h = hint(
        dir.join("proj")
            .join("crates")
            .join("x.rs")
            .to_str()
            .unwrap(),
    );
    assert_eq!(h, None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_hint_rides_on_the_error_the_caller_already_wrote() {
    let dir = tree("wrap");
    let path = dir.join("proj").join("mian.rs");
    let p = path.to_str().unwrap();
    let out = with_hint("read", p, "No such file or directory (os error 2)");
    assert!(out.starts_with(&format!("read {p}: No such file")), "{out}");
    assert!(
        out.contains("did you mean \u{201c}main.rs\u{201d}?"),
        "{out}"
    );
    // No hint to give: the caller's message stands alone, unchanged.
    let out = with_hint("read", "/nonexistent-root-xyz/a.rs", "nope");
    assert_eq!(out, "read /nonexistent-root-xyz/a.rs: nope");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_bare_filename_is_looked_for_where_the_broker_is_standing() {
    // The commonest shape of all, and an empty parent is not a missing one.
    let dir = tree("bare");
    let cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.join("proj")).unwrap();
    let h = hint("mian.rs");
    std::env::set_current_dir(cwd).unwrap();
    let h = h.expect("a hint for a bare name");
    assert!(h.starts_with("the working directory has no"), "{h}");
    assert!(h.contains("did you mean \u{201c}main.rs\u{201d}?"), "{h}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_deep_directory_is_named_by_its_tail_not_its_whole_path() {
    let dir = tree("deep");
    std::fs::write(dir.join("proj").join("crates").join("lib.rs"), "x").unwrap();
    let h = hint(
        dir.join("proj")
            .join("crates")
            .join("zzz")
            .to_str()
            .unwrap(),
    )
    .expect("a hint");
    assert!(h.contains("\u{2026}/proj/crates"), "{h}");
    assert!(
        !h.contains("/var/folders") && !h.contains(dir.to_str().unwrap()),
        "{h}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
