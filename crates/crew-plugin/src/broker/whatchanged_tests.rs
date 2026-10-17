use std::path::{Path, PathBuf};
use std::process::Command;

use super::{about_breakage, block_at, whole_files, CAP};

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(ok.success(), "git {args:?}");
}

/// A throwaway repo with `lib.rs` committed, isolated per test.
fn repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-whatchanged-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@t"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(
        dir.join("lib.rs"),
        "fn lum() -> f32 {\n    0.7152 * g + 0.0722 * b\n}\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "luminance"]);
    dir
}

#[test]
fn a_task_about_breakage_is_recognised_and_others_are_not() {
    for t in [
        "cargo test started failing after my last change",
        "the build broke",
        "Why does it panic on startup?",
        "this no longer renders",
    ] {
        assert!(about_breakage(t), "{t}");
    }
    for t in ["add a /foo command", "explain what crew-render does"] {
        assert!(!about_breakage(t), "{t}");
    }
}

#[test]
fn a_dirty_tree_puts_its_uncommitted_diff_first() {
    let dir = repo("dirty");
    std::fs::write(
        dir.join("lib.rs"),
        "fn lum() -> f32 {\n    0.7152 * b + 0.0722 * g\n}\n",
    )
    .unwrap();
    let b = block_at(&dir).unwrap();
    assert!(b.starts_with("WHAT CHANGED (uncommitted)"), "{b}");
    assert!(b.contains("lib.rs | 2 +-"), "the stat: {b}");
    assert!(
        b.contains("-    0.7152 * g + 0.0722 * b"),
        "the old line: {b}"
    );
    assert!(
        b.contains("+    0.7152 * b + 0.0722 * g"),
        "the new line: {b}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_clean_tree_offers_the_last_commit_and_says_so() {
    let dir = repo("clean");
    let b = block_at(&dir).unwrap();
    assert!(b.contains("the last commit; the tree is clean"), "{b}");
    assert!(b.contains("luminance"), "the commit's subject: {b}");
    assert!(b.contains("+    0.7152 * g + 0.0722 * b"), "{b}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn outside_a_repository_there_is_nothing() {
    let dir = std::env::temp_dir().join(format!("crew-whatchanged-none-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(block_at(&dir).is_none());
    std::fs::remove_dir_all(&dir).ok();
}

fn file(name: &str, lines: usize) -> String {
    let body: String = (0..lines).map(|i| format!("+line {i}\n")).collect();
    format!("diff --git a/{name} b/{name}\n@@ -0,0 +1,{lines} @@\n{body}")
}

#[test]
fn whole_files_are_kept_while_they_fit_and_the_rest_are_named() {
    let patch = [file("a.rs", 10), file("b.rs", 10), file("c.rs", 400)].join("\n");
    let out = whole_files(&patch, CAP);
    assert!(out.contains("+line 9\n"), "a.rs whole");
    assert!(out.contains("diff --git a/b.rs b/b.rs"), "b.rs whole");
    assert!(out.ends_with("… not shown: c.rs\n"), "{out}");
    assert!(out.len() <= CAP + 40);
}

#[test]
fn one_file_too_big_on_its_own_is_cut_at_a_line() {
    let out = whole_files(&file("big.rs", 2_000), CAP);
    assert!(out.len() <= CAP + 60);
    assert!(
        out.ends_with("… (cut: this file's diff is longer)\n"),
        "{out}"
    );
    let last = out.lines().rev().nth(1).unwrap();
    assert!(last.starts_with("+line "), "cut on a whole line: {last:?}");
}
