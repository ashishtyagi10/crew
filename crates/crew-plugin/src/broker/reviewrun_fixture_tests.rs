//! A real repository and a fake reviewer that counts its calls and the
//! effort each one ran at, for `reviewrun_tests`.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use super::super::run;
use crate::broker::adapter::Adapter;
use crate::broker::reviewlevel::Level;
use crew_hive::Effort;

/// Answers from a script, in order, and records each prompt with the
/// effort the turn would have asked for.
struct Reviewer {
    replies: Mutex<Vec<String>>,
    seen: Mutex<Vec<(String, Option<Effort>)>>,
    none: PathBuf,
}

impl Adapter for Reviewer {
    fn name(&self) -> &str {
        "reviewer"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        let effort = crate::broker::routeeffort::for_turn_at(&self.none, "reviewer");
        self.seen.lock().unwrap().push((body.to_string(), effort));
        let mut r = self.replies.lock().unwrap();
        Ok(if r.is_empty() {
            String::new()
        } else {
            r.remove(0)
        })
    }
}

fn reviewer(dir: &Path, replies: &[&str]) -> Reviewer {
    Reviewer {
        replies: Mutex::new(replies.iter().map(|s| s.to_string()).collect()),
        seen: Mutex::new(Vec::new()),
        none: dir.join("no-specialists"),
    }
}

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

/// `foo` defined in lib.rs (12 lines) and called from app.rs. Numbered,
/// not timed: two tests at one level start within the clock's resolution.
fn repo(tag: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "crew-reviewrun-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@t"]);
    git(&dir, &["config", "user.name", "t"]);
    let lib = format!(
        "pub fn foo(a: u8) -> u8 {{\n    a\n}}\n{}",
        "//\n".repeat(9)
    );
    std::fs::write(dir.join("lib.rs"), lib).unwrap();
    std::fs::write(dir.join("app.rs"), "fn run() {\n    let x = foo(1);\n}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "init"]);
    dir
}

pub(super) const DIFF: &str = "--- a/lib.rs\n+++ b/lib.rs\n@@ -1,3 +1,3 @@\n\
     -pub fn foo(a: u8) -> u8 {\n+pub fn foo(a: u8, b: u8) -> u8 {\n     a\n }\n";

pub(super) const TWO: &str = "- blocker — lib.rs:1 — arity changed\n\
                   - warn — lib.rs:2 — b unused\n\nverdict: 1 blocker, 1 warn";

pub(super) fn ran(
    level: Level,
    replies: &[&str],
) -> (super::super::Ran, Vec<(String, Option<Effort>)>) {
    let dir = repo(&format!("{level:?}"));
    let a = reviewer(&dir, replies);
    let _outer = crate::broker::routeeffort::enter(Some(Effort::High));
    let r = run(level, &dir, DIFF, &a, Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
    (r, a.seen.into_inner().unwrap())
}
