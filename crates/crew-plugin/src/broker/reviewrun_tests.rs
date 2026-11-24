//! What each level costs and sends, against a real repository and a fake
//! reviewer that counts its calls and the effort each one ran at.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use super::run;
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

/// `foo` defined in lib.rs (12 lines) and called from app.rs.
fn repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-reviewrun-{tag}-{}-{}",
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

const DIFF: &str = "--- a/lib.rs\n+++ b/lib.rs\n@@ -1,3 +1,3 @@\n\
     -pub fn foo(a: u8) -> u8 {\n+pub fn foo(a: u8, b: u8) -> u8 {\n     a\n }\n";

const TWO: &str = "- blocker — lib.rs:1 — arity changed\n\
                   - warn — lib.rs:2 — b unused\n\nverdict: 1 blocker, 1 warn";

fn ran(level: Level, replies: &[&str]) -> (super::Ran, Vec<(String, Option<Effort>)>) {
    let dir = repo(&format!("{level:?}"));
    let a = reviewer(&dir, replies);
    let _outer = crate::broker::routeeffort::enter(Some(Effort::High));
    let r = run(level, &dir, DIFF, &a, Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
    (r, a.seen.into_inner().unwrap())
}

#[test]
fn a_quick_look_is_one_cheap_call_on_the_diff_alone() {
    let (r, seen) = ran(Level::Quick, &[TWO]);
    assert_eq!(seen.len(), 1);
    let (body, effort) = &seen[0];
    assert!(!body.contains("CALLERS"), "{body}");
    assert!(body.contains("at most 5 findings"), "{body}");
    assert_eq!(*effort, Some(Effort::Low), "over the route's high");
    let t = r.reply.unwrap().unwrap();
    assert!(
        t.text.starts_with("quick look · 2 findings\n"),
        "{}",
        t.text
    );
}

#[test]
fn a_standard_review_is_today_s_one_call_and_text() {
    let (r, seen) = ran(Level::Standard, &[TWO]);
    assert_eq!(seen.len(), 1);
    let (body, effort) = &seen[0];
    assert!(
        body.contains("CALLERS") && body.contains("app.rs:2:"),
        "{body}"
    );
    assert_eq!(*effort, Some(Effort::High), "the route's level stands");
    let t = r.reply.unwrap().unwrap();
    assert_eq!(t.text, TWO, "no label, nothing changed");
    assert!(r.notes.is_empty());
}

#[test]
fn a_deep_review_reads_callers_and_gives_each_finding_a_second_look() {
    let (r, seen) = ran(Level::Deep, &[TWO, "1 keep\n2 drop — b is used on line 2"]);
    assert_eq!(seen.len(), 2, "today's call plus the second look");
    assert!(seen[0].0.contains("CALLERS") && seen[0].0.contains("DEEP REVIEW"));
    assert!(
        seen[1].0.contains("FINDINGS:\n1. - blocker — lib.rs:1"),
        "{}",
        seen[1].0
    );
    assert!(seen.iter().all(|(_, e)| *e == Some(Effort::High)));
    let t = r.reply.unwrap().unwrap();
    assert!(
        t.text.starts_with("deep review · 1 finding\n"),
        "{}",
        t.text
    );
    assert!(t.text.ends_with("verdict: 1 blocker"), "{}", t.text);
    assert!(!t.text.contains("b unused"), "{}", t.text);
    assert_eq!(
        t.dropped,
        ["lib.rs:2 withdrawn on a second look — b is used on line 2"]
    );
}

#[test]
fn a_deep_review_with_nothing_found_makes_no_second_call() {
    let (r, seen) = ran(Level::Deep, &["no findings"]);
    assert_eq!(seen.len(), 1);
    let t = r.reply.unwrap().unwrap();
    assert!(
        t.text.starts_with("deep review · no findings"),
        "{}",
        t.text
    );
}

#[test]
fn a_second_look_that_does_not_answer_leaves_the_findings_unverified() {
    let (r, seen) = ran(Level::Deep, &[TWO, "sure, all good"]);
    assert_eq!(seen.len(), 2);
    let t = r.reply.unwrap().unwrap();
    assert!(t.text.starts_with("deep review · 2 findings"), "{}", t.text);
    assert_eq!(r.notes.len(), 1);
    assert!(r.notes[0].contains("unverified"), "{:?}", r.notes);
}

#[test]
fn a_quick_look_that_overruns_shows_five() {
    let seven: Vec<String> = (1..=7)
        .map(|n| format!("- nit — lib.rs:{n} — thing {n}"))
        .collect();
    let (r, _) = ran(Level::Quick, &[&seven.join("\n")]);
    let t = r.reply.unwrap().unwrap();
    assert!(t.text.starts_with("quick look · 5 findings"), "{}", t.text);
    assert!(t.text.ends_with("verdict: 5 nit"), "{}", t.text);
}
