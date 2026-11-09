//! The S8/S9 probe kit as a scratch repository: `add(a, b)` becomes
//! `add(a, b, c)` in lib.rs; app.rs and cli.rs still pass two, main.rs was
//! updated to three. Only the two left behind are named.
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{call_args, note, signature};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A committed repo holding `files`, for a test named `tag`.
pub(crate) fn repo(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-stale-{tag}-{}-{:?}",
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
    for (path, text) in files {
        std::fs::write(dir.join(path), text).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "init"]);
    dir
}

/// `path` rewritten to `text`, and the patch that made (`git diff`).
pub(crate) fn edit(dir: &Path, path: &str, text: &str) -> String {
    std::fs::write(dir.join(path), text).unwrap();
    git(dir, &["diff"])
}

pub(crate) const LIB: &str = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
pub(crate) const LIB3: &str = "pub fn add(a: i32, b: i32, c: i32) -> i32 {\n    a + b + c\n}\n";

pub(crate) fn kit(tag: &str) -> PathBuf {
    repo(
        tag,
        &[
            ("lib.rs", LIB),
            (
                "app.rs",
                "use crate::add;\nfn run() {\n    let x = 0;\n    let y = add(x, 1);\n}\n",
            ),
            ("cli.rs", "fn go() {\n    add(2, 3);\n}\n"),
            ("main.rs", "fn main() {\n    add(1, 2);\n}\n"),
        ],
    )
}

#[test]
fn a_signature_change_names_every_caller_it_left_behind() {
    let dir = kit("sig");
    std::fs::write(dir.join("main.rs"), "fn main() {\n    add(1, 2, 3);\n}\n").unwrap();
    let patch = edit(&dir, "lib.rs", LIB3);
    let n = note(&dir, &patch).expect("two callers left behind");
    assert_eq!(
        n,
        "! 2 callers of add() not updated: app.rs:4 \u{b7} cli.rs:2"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_body_change_or_every_caller_updated_says_nothing() {
    let dir = kit("body");
    let body = edit(
        &dir,
        "lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 {\n    b + a\n}\n",
    );
    assert_eq!(note(&dir, &body), None, "same arity: nothing to name");
    for (f, t) in [
        ("app.rs", "fn run() {\n    add(0, 1, 2);\n}\n"),
        ("cli.rs", "fn go() {\n    add(2, 3, 4);\n}\n"),
        ("main.rs", "fn main() {\n    add(1, 2, 3);\n}\n"),
    ] {
        std::fs::write(dir.join(f), t).unwrap();
    }
    let patch = edit(&dir, "lib.rs", LIB3);
    assert_eq!(note(&dir, &patch), None, "every caller passes three");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn signatures_count_what_a_call_passes() {
    let s = signature;
    assert_eq!(
        s("pub fn add(a: i32, b: i32) -> i32 {"),
        Some(("add".into(), 2))
    );
    assert_eq!(
        s("    pub(crate) fn put<K: Ord, V>(&mut self, m: HashMap<K, V>, f: impl Fn(u8) -> u8) {"),
        Some(("put".into(), 2)),
        "self is passed before the dot; generics and Fn(..) -> are one param each"
    );
    assert_eq!(s("def area(self, w, h):"), Some(("area".into(), 2)));
    assert_eq!(s("fn wrapped("), None, "an open list is not guessed at");
    assert_eq!(s("let defined = 1;"), None);
    assert_eq!(call_args("    let y = add(x, f(1, 2));", "add"), Some(2));
    assert_eq!(call_args("    let y = madd(x);", "add"), None);
    assert_eq!(call_args("    s.add(\"a, b\");", "add"), Some(1));
    assert_eq!(call_args("    add(", "add"), None);
}
