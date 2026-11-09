//! Findings are checked against the tree before the pane shows them.
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

/// A tree with `lib.rs` of 40 lines and nothing else.
fn forty(file: &str) -> Cited {
    if file == "lib.rs" {
        Cited::Lines(40)
    } else {
        Cited::Missing
    }
}

/// Loop 39's S9 live quirk: two blockers, then `no findings`.
#[test]
fn a_stray_no_findings_under_findings_is_dropped() {
    let r = "- blocker — lib.rs:4 — unwrap on None\n\
             - blocker — lib.rs:9 — off by one\n\nno findings";
    let t = tidy(r, &forty);
    assert!(!t.text.contains("no findings"), "{}", t.text);
    assert!(t.text.ends_with("verdict: 2 blocker"), "{}", t.text);
    assert!(t.dropped.is_empty());
}

#[test]
fn a_line_past_the_end_and_a_ghost_file_are_dropped_and_said() {
    let r = "- blocker — `lib.rs:400` — reads past the end\n\
             - warn — ghost.rs:9 — never closed\n\
             - warn — lib.rs:12 — slow loop\n\
             - nit — lib.rs — naming\n\nverdict: 1 blocker, 2 warn, 1 nit";
    let t = tidy(r, &forty);
    assert!(
        !t.text.contains("lib.rs:400") && !t.text.contains("ghost.rs"),
        "{}",
        t.text
    );
    assert!(
        t.text.contains("lib.rs:12") && t.text.contains("naming"),
        "{}",
        t.text
    );
    // The model's count said 4; two were kept.
    assert!(t.text.ends_with("verdict: 1 warn, 1 nit"), "{}", t.text);
    assert_eq!(
        dropped_note(&t.dropped).unwrap(),
        "2 findings dropped — cited lib.rs:400, past its end (40 lines) · \
         cited ghost.rs:9, no such file"
    );
}

#[test]
fn the_verdict_is_recounted_even_when_nothing_is_dropped() {
    let r = "1. **blocker** — lib.rs:3 — x\n2. warning — lib.rs:5 — y\n\nVerdict: 3 blocker";
    let t = tidy(r, &forty);
    assert!(t.text.ends_with("verdict: 1 blocker, 1 warn"), "{}", t.text);
    assert!(!t.text.contains("3 blocker"), "{}", t.text);
}

#[test]
fn every_finding_dropped_leaves_no_findings_and_a_clean_reply_is_untouched() {
    let t = tidy("- blocker — ghost.rs:1 — x\nverdict: 1 blocker", &forty);
    assert_eq!(t.text, "no findings");
    assert_eq!(t.dropped.len(), 1);
    for clean in ["no findings", "Looks fine overall.\n- the tests read well"] {
        assert_eq!(tidy(clean, &forty).text, clean);
    }
    // A file the lookup cannot see is kept, not judged.
    let t = tidy("- warn — far.rs:900 — y", &|_| Cited::Unknown);
    assert!(
        t.dropped.is_empty() && t.text.contains("far.rs:900"),
        "{}",
        t.text
    );
}

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(ok.success());
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-reviewfindings-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
    ));
    std::fs::create_dir_all(dir.join("src")).unwrap();
    git(&dir, &["init", "-q"]);
    let body: String = (1..=40).map(|i| format!("// {i}\n")).collect();
    std::fs::write(dir.join("src/lib.rs"), body).unwrap();
    dir
}

/// Against a real tree: `lib.rs` resolves to `src/lib.rs` by suffix, an
/// untracked file still counts, and a file the diff deleted is not "missing".
#[test]
fn tidy_in_looks_cited_files_up_in_the_tree() {
    let dir = scratch();
    let diff = "--- a/gone.rs\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";
    let r = "- blocker — lib.rs:40 — a\n- blocker — src/lib.rs:41 — b\n\
             - warn — gone.rs:1 — c\n- nit — ghost.rs:2 — d\nno findings";
    let t = tidy_in(&dir, diff, r);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        t.text.contains("lib.rs:40") && t.text.contains("gone.rs:1"),
        "{}",
        t.text
    );
    assert!(
        !t.text.contains(":41") && !t.text.contains("ghost.rs"),
        "{}",
        t.text
    );
    assert!(!t.text.contains("no findings"), "{}", t.text);
    assert!(t.text.ends_with("verdict: 1 blocker, 1 warn"), "{}", t.text);
    assert_eq!(t.dropped.len(), 2, "{:?}", t.dropped);
}
