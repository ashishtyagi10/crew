use super::*;
use crate::broker::sysgrep::{glob, grep};

fn tree(files: &[(&str, &str)]) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("crew-grepfiles-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for (rel, text) in files {
        let p = d.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    d
}

fn git(dir: &Path, args: &[&str]) {
    let ok = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    assert!(ok, "git {args:?} failed in {}", dir.display());
}

/// A project whose `.gitignore` names its generated code, with a hit in its
/// own code and one in the generated.
const PROJECT: &[(&str, &str)] = &[
    (".gitignore", "gen/\n"),
    ("src/a.rs", "fn needle() {}\n"),
    ("gen/b.rs", "fn needle() {}\n"),
];

fn search(d: &Path) -> String {
    grep(&serde_json::json!({"pattern": "needle", "path": d})).unwrap()
}

/// In a work tree the list is git's: what `.gitignore` names is not searched,
/// and a file git has not been told about yet, but does not ignore, is.
#[test]
fn a_work_tree_is_searched_as_git_lists_it() {
    let d = tree(PROJECT);
    git(&d, &["init", "-q"]);
    git(&d, &["add", ".gitignore", "src/a.rs"]);
    std::fs::write(d.join("src/new.rs"), "needle();\n").unwrap();
    assert_eq!(
        search(&d),
        "src/a.rs:1: fn needle() {}\nsrc/new.rs:1: needle();"
    );
    let names = glob(&serde_json::json!({"pattern": "*.rs", "path": d})).unwrap();
    assert_eq!(names, "src/a.rs\nsrc/new.rs");
    // Named on purpose, an ignored directory is searched after all.
    let named = grep(&serde_json::json!({"pattern": "needle", "path": d.join("gen")})).unwrap();
    assert_eq!(named, "b.rs:1: fn needle() {}");
}

/// With no repository there is nothing to ask: the walk answers, skipping the
/// fixed directories and hidden ones, and knowing nothing of `.gitignore`.
#[test]
fn outside_a_work_tree_the_walk_answers() {
    let mut files = PROJECT.to_vec();
    files.extend([
        ("target/c.rs", "fn needle() {}\n"),
        ("node_modules/d.js", "needle\n"),
        (".cache/e.rs", "needle\n"),
    ]);
    let d = tree(&files);
    assert_eq!(listed(&d), None, "{} is inside a repository", d.display());
    assert_eq!(
        search(&d),
        "gen/b.rs:1: fn needle() {}\nsrc/a.rs:1: fn needle() {}"
    );
}

/// Git's list, sorted as the walk goes: `a/` before `a-b/`, though a sort of
/// whole paths puts `-` before `/`.
#[test]
fn the_listing_comes_in_the_walks_order() {
    let d = tree(&[("a-b/x", ""), ("a/x", ""), ("a/y/z", ""), ("b", "")]);
    git(&d, &["init", "-q"]);
    let walk: Vec<String> = walked(&d).into_iter().map(|(r, _)| r).collect();
    assert_eq!(walk, ["a/x", "a/y/z", "a-b/x", "b"]);
    assert_eq!(listed(&d), Some(walk));
}

/// A small search reads exactly as it did before hits got context and a
/// budget, byte for byte, in a plain directory and in a git work tree alike —
/// the order included, which is the walk's (`a/` before `a-b/`), not a sort
/// of whole paths.
#[test]
fn a_small_search_reads_as_it_always_has() {
    let long = format!("    draw {}", "x".repeat(250));
    let files = [
        ("a-b/x.rs", "draw here\n".to_string()),
        (
            "a/x.rs",
            "fn f() {\n    let draw = 1;\r\n\tdraw();\n}\n".to_string(),
        ),
        ("z.md", format!("{long}\n")),
    ];
    let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
    let want_grep = format!(
        "a/x.rs:2: let draw = 1;\na/x.rs:3: draw();\na-b/x.rs:1: draw here\nz.md:1: draw {}",
        "x".repeat(195)
    );
    let plain = tree(&files);
    let repo = tree(&files);
    git(&repo, &["init", "-q"]);
    git(&repo, &["add", "a", "z.md"]); // a-b/x.rs stays untracked
    for d in [&plain, &repo] {
        let out = grep(&serde_json::json!({"pattern": "draw", "path": d})).unwrap();
        assert_eq!(out, want_grep, "{}", d.display());
        let out = glob(&serde_json::json!({"pattern": "*.rs", "path": d})).unwrap();
        assert_eq!(out, "a/x.rs\na-b/x.rs", "{}", d.display());
    }
}

/// A build directory the project never ignored is still skipped: git lists
/// it as "others", and the fixed list catches what `.gitignore` forgot.
#[test]
fn an_unignored_build_dir_is_still_skipped() {
    let d = tree(&[
        ("src/a.rs", "fn needle() {}\n"),
        ("target/debug/b.rs", "fn needle() {}\n"),
    ]);
    git(&d, &["init", "-q"]);
    assert_eq!(search(&d), "src/a.rs:1: fn needle() {}");
    let _ = std::fs::remove_dir_all(&d);
}
