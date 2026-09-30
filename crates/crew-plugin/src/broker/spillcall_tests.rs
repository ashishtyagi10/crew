//! Each caller of `spill`: the pointer only when the fitting dropped
//! something, the result still within `RUN_FIT`, the words that said what
//! was left out still there, and the file holding everything.
use std::path::Path;

use super::file::DIR;
use super::spilltest::guard;
use super::RUN_FIT;
use serde_json::json;

/// The one spill under `root`, read back, and the path the pointer names.
fn only_spill(root: &Path) -> (String, String) {
    let names: Vec<String> = std::fs::read_dir(root.join(DIR))
        .expect("a spill directory")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".txt"))
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
    let rel = format!("{DIR}/{}", names[0]);
    (std::fs::read_to_string(root.join(&rel)).unwrap(), rel)
}

/// `out` is within budget and ends with the pointer to `rel`.
fn points_at(out: &str, rel: &str) {
    assert!(out.len() <= RUN_FIT, "{} bytes", out.len());
    let last = out.rsplit('\n').next().unwrap();
    assert!(last.starts_with("\u{2026} full output ("), "{last}");
    assert!(last.contains(&format!("saved to {rel} ")), "{last}");
}

#[cfg(unix)]
#[test]
fn a_long_run_is_fitted_and_saved_whole() {
    let g = guard("run");
    let t = std::time::Duration::from_secs(10);
    let out = super::super::sysrun::run_with("seq 1 5000", t).unwrap();
    let (body, rel) = only_spill(g.dir());
    points_at(&out, &rel);
    assert!(out.starts_with("exit 0\n1\n"), "{out}");
    assert!(out.contains("lines of output cut \u{2014} the end is below)\n"));
    assert!(
        out.contains("\n5000\n\u{2026} full output (5,001 lines"),
        "{out}"
    );
    let stdout: String = (1..=5000).map(|i| format!("{i}\n")).collect();
    assert_eq!(body, super::super::runfit::whole(0, &stdout, "", false));
    // A short one reads exactly as it always has, and saves nothing.
    let short = super::super::sysrun::run_with("echo hi", t).unwrap();
    assert_eq!(short, "exit 0\nhi\n");
    assert_eq!(only_spill(g.dir()).1, rel);
}

#[test]
fn a_long_diff_is_fitted_and_saved_whole() {
    let g = guard("git");
    let repo = g.dir().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let sh = |args: &[&str]| {
        let ok = std::process::Command::new("git")
            .args(args)
            .current_dir(&repo)
            .status()
            .unwrap();
        assert!(ok.success(), "git {args:?}");
    };
    sh(&["init", "-q"]);
    sh(&["config", "user.email", "ada@example.com"]);
    sh(&["config", "user.name", "Ada Lovelace"]);
    sh(&["config", "commit.gpgsign", "false"]);
    std::fs::write(repo.join("a.txt"), "one\n").unwrap();
    sh(&["add", "-A"]);
    sh(&["commit", "-q", "-m", "one"]);
    let status = super::super::sysgit::git_in(&repo, &json!({"cmd": "status"})).unwrap();
    assert!(!status.contains("full output"), "{status}");
    assert!(
        !g.dir().join(".crew").exists(),
        "a short answer saves nothing"
    );
    let lines: String = (1..=3000).map(|i| format!("added line {i}\n")).collect();
    std::fs::write(repo.join("a.txt"), lines).unwrap();
    let out = super::super::sysgit::git_in(&repo, &json!({"cmd": "diff"})).unwrap();
    let (body, rel) = only_spill(g.dir());
    points_at(&out, &rel);
    assert!(
        out.contains("1 file changed"),
        "the --stat head is kept: {out}"
    );
    assert!(out.contains("lines of output cut"), "{out}");
    assert!(body.contains("\n+added line 1500\n"), "the middle is saved");
    assert!(body.ends_with("+added line 3000\n"));
}

#[test]
fn every_hit_left_out_is_saved_and_the_count_stays() {
    let g = guard("grep");
    let src = g.dir().join("src");
    for f in 0..5 {
        let text: String = (0..100)
            .map(|i| format!("hit {i:03} of file {f}\n"))
            .collect();
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join(format!("f{f}.txt")), text).unwrap();
    }
    let out = super::super::sysgrep::grep(&json!({"pattern": "^hit", "path": src})).unwrap();
    let (body, rel) = only_spill(g.dir());
    points_at(&out, &rel);
    let count = out.lines().rev().nth(1).unwrap();
    assert!(
        count.starts_with("\u{2026} ") && count.contains(" more hits in "),
        "{count}"
    );
    assert_eq!(body.lines().count(), 500);
    assert_eq!(body.lines().nth(399), Some("f3.txt:100: hit 099 of file 3"));
    // A search that fits is what it always was.
    let one = super::super::sysgrep::grep(&json!({"pattern": "hit 042 of file 4", "path": src}));
    assert_eq!(one.unwrap(), "f4.txt:43: hit 042 of file 4");
}

#[test]
fn every_path_left_out_is_saved() {
    let g = guard("glob");
    let src = g.dir().join("src");
    std::fs::create_dir_all(&src).unwrap();
    for i in 0..600 {
        std::fs::write(src.join(format!("module_{i:03}.rs")), "").unwrap();
    }
    let out = super::super::sysgrep::glob(&json!({"pattern": "*.rs", "path": src})).unwrap();
    let (body, rel) = only_spill(g.dir());
    points_at(&out, &rel);
    assert!(out
        .lines()
        .rev()
        .nth(1)
        .unwrap()
        .contains(" more paths left out"));
    assert_eq!(body.lines().count(), 600);
    assert_eq!(body.lines().last(), Some("module_599.rs"));
}
