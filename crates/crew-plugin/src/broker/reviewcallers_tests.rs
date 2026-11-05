//! The callers block against a real repository: a signature change to
//! `pub fn foo(a)` must bring back both call sites by file:line, and none of
//! the definition, the import or the comment that also say `foo`.
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{changed_symbols, section};
use crate::broker::review::review_prompt;

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

/// A repo with `foo` defined in lib.rs and called from app.rs and cli.rs.
fn temp_repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-reviewcallers-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@t"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join("lib.rs"), "pub fn foo(a: u8) -> u8 {\n    a\n}\n").unwrap();
    std::fs::write(
        dir.join("app.rs"),
        "use crate::lib::foo;\n// foo is the helper\nfn run() {\n    let x = foo(1);\n}\n",
    )
    .unwrap();
    std::fs::write(dir.join("cli.rs"), "fn go() {\n\n    foo(2);\n}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "init"]);
    dir
}

const SIGNATURE_DIFF: &str = "--- a/lib.rs\n+++ b/lib.rs\n@@ -1,3 +1,3 @@\n\
     -pub fn foo(a: u8) -> u8 {\n+pub fn foo(a: u8, b: u8) -> u8 {\n     a\n }\n";

#[test]
fn a_signature_change_brings_back_every_call_site_by_file_and_line() {
    let dir = temp_repo("sig");
    let s = section(&dir, SIGNATURE_DIFF);
    assert!(s.contains("CALLERS"), "{s}");
    assert!(s.contains("app.rs:4:"), "first caller by file:line: {s}");
    assert!(s.contains("cli.rs:3:"), "second caller by file:line: {s}");
    assert!(
        !s.contains("lib.rs:1:"),
        "the definition is not a caller: {s}"
    );
    assert!(!s.contains("app.rs:1:"), "the import is not a caller: {s}");
    assert!(!s.contains("app.rs:2:"), "a comment is not a caller: {s}");
    let p = review_prompt(SIGNATURE_DIFF, &s);
    assert!(p.contains("cli.rs:3:") && p.contains("DIFF:"), "{p}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_diff_that_changes_no_definition_adds_nothing() {
    let dir = temp_repo("none");
    let diff = "--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-one\n+two\n";
    assert_eq!(section(&dir, diff), "");
    assert!(!review_prompt(diff, "").contains("CALLERS"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn signature_edits_lead_then_the_bodies_named_by_hunk_headers() {
    let diff = "@@ -10,2 +10,2 @@ pub(crate) fn outer_body(x: u8) {\n\
                -    let y = 1;\n+    let y = 2;\n\
                +pub fn renamed<T>(t: T) {}\n\
                -fn new() -> Self {\n+fn test_thing() {}\n";
    assert_eq!(changed_symbols(diff), ["renamed", "outer_body"]);
}
