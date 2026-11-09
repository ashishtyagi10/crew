//! A fake reviewer CLI, known to crew by nothing but its manifest, run
//! against a scratch repository: what it writes during the review must be
//! gone afterwards and said so, and a reviewer that only reads is not
//! mentioned at all.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::call;
use crate::broker::adapter::Adapter;
use crate::broker::plugins::{from_manifest, Manifest};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-reviewguard-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(dir.join("repo")).unwrap();
    dir
}

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

/// A repo whose `lib.rs` the user has edited and not committed: the diff a
/// review is asked about, which must survive the review untouched.
fn repo(root: &Path) -> PathBuf {
    let repo = root.join("repo");
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "t@t"]);
    git(&repo, &["config", "user.name", "t"]);
    std::fs::write(repo.join("lib.rs"), "fn add(a: i32) {}\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "init"]);
    std::fs::write(repo.join("lib.rs"), "fn add(a: i32, b: i32) {}\n").unwrap();
    repo
}

/// A manifest agent whose command is a shell script outside the repo.
#[cfg(unix)]
fn reviewer(root: &Path, script: &str) -> impl Adapter {
    let path = root.join("myreviewer");
    std::fs::write(&path, format!("#!/bin/sh\n{script}\necho no findings\n")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    from_manifest(Manifest {
        name: "myreviewer".into(),
        command: path.to_string_lossy().into_owned(),
        args: vec![],
        role: String::new(),
    })
    .unwrap()
}

#[cfg(unix)]
#[test]
fn a_reviewer_with_no_read_only_mode_has_its_edits_put_back_and_named() {
    let root = scratch("edits");
    let repo = repo(&root);
    let lib = repo.join("lib.rs");
    let script = format!(
        "echo '// fixed' >> '{}'\necho 'x' > '{}'",
        lib.display(),
        repo.join("new.rs").display()
    );
    let a = reviewer(&root, &script);
    assert!(!a.hands_off(), "crew knows no read-only mode for it");
    let (reply, note) = call(&repo, &a, "review", Duration::from_secs(10));
    assert_eq!(reply.unwrap().trim(), "no findings");
    let note = note.expect("the edit is said");
    assert!(note.starts_with("! the reviewer edited "), "{note}");
    assert!(note.contains("lib.rs") && note.contains("new.rs"), "{note}");
    assert!(note.ends_with("restored"), "{note}");
    assert_eq!(
        std::fs::read_to_string(&lib).unwrap(),
        "fn add(a: i32, b: i32) {}\n",
        "the user's uncommitted edit is back, the reviewer's gone"
    );
    assert!(!repo.join("new.rs").exists(), "the file it made is removed");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn a_reviewer_that_only_reads_is_not_mentioned() {
    let root = scratch("reads");
    let repo = repo(&root);
    let a = reviewer(
        &root,
        &format!("cat '{}' >/dev/null", repo.join("lib.rs").display()),
    );
    let (reply, note) = call(&repo, &a, "review", Duration::from_secs(10));
    assert!(reply.is_ok());
    assert_eq!(note, None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_manifest_naming_a_known_cli_reviews_in_its_read_only_mode() {
    let a = from_manifest(Manifest {
        name: "oc".into(),
        command: "opencode".into(),
        args: vec!["run".into(), "{}".into()],
        role: String::new(),
    })
    .unwrap();
    assert!(a.hands_off());
}
