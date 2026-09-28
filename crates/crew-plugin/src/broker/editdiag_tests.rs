use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crew_lsp::{Diagnostic, Position, Range, Severity};
use serde_json::json;

use super::*;
use crate::broker::session::sessiontest::sys_surface_diagnosing;

const BODY: &str = "fn a() {}\n";

fn diag(line: u32, character: u32, severity: Severity, message: &str) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character },
            ..Range::default()
        },
        severity,
        message: message.into(),
        source: Some("rustc".into()),
    }
}

/// rust-analyzer's `cargo check` error, two lines as it sends them, and the
/// note it makes at 1:8, exactly.
const MISMATCH: &str = "mismatched types\nexpected `i32`, found `&str`";
const NOTE: &str =
    "diagnostics now: 1 new error\n  1:8 error: mismatched types \u{2014} expected `i32`, found `&str`";

fn mismatch() -> Diagnostic {
    diag(0, 7, Severity::Error, MISMATCH)
}

/// A source that serves what `serves` says, takes `delay` over each ask and
/// answers `list`, writing down every question it is asked, in order.
struct Fake {
    serves: bool,
    delay: Duration,
    list: Vec<Diagnostic>,
    log: Arc<Mutex<Vec<String>>>,
}

impl Source for Fake {
    fn serves(&self, path: &Path) -> bool {
        self.note("serves", path);
        self.serves
    }
    fn diagnostics(&mut self, path: &Path, _: Duration) -> Option<Vec<Diagnostic>> {
        self.note("asked", path);
        std::thread::sleep(self.delay);
        Some(self.list.clone())
    }
}

impl Fake {
    fn note(&self, what: &str, path: &Path) {
        let name = path.file_name().unwrap().to_string_lossy();
        self.log.lock().unwrap().push(format!("{what} {name}"));
    }
}

fn fake(serves: bool, delay: Duration, list: Vec<Diagnostic>) -> (Shared, Arc<Mutex<Vec<String>>>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let src = Fake {
        serves,
        delay,
        list,
        log: Arc::clone(&log),
    };
    (Arc::new(Mutex::new(src)), log)
}

/// `name` holding [`BODY`], in a directory of its own: numbered, since the
/// clock's microseconds gave two parallel tests one directory to delete.
fn file(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("crew-editdiag-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, BODY).unwrap();
    path
}

/// One edit through a surface asking `src`, how long it took, and what the
/// same edit on the same text answers with nobody asked.
fn edit(src: Shared, path: &Path) -> (String, Duration, String) {
    let args = json!({"path": path, "old": "fn a()", "new": "fn a(x: i32)"}).to_string();
    let t = sys_surface_diagnosing(src);
    let started = Instant::now();
    let got = t.call("sys", "edit", &args).unwrap();
    let took = started.elapsed();
    std::fs::write(path, BODY).unwrap();
    let plain = crate::broker::systools::call("edit", &args).unwrap();
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
    (got, took, plain)
}

#[test]
fn an_edit_that_left_an_error_ends_with_it() {
    let (src, log) = fake(true, Duration::ZERO, vec![mismatch()]);
    let (got, _, plain) = edit(src, &file("a.rs"));
    assert_eq!(got, format!("{plain}\n\n{NOTE}"));
    assert_eq!(*log.lock().unwrap(), ["serves a.rs", "asked a.rs"]);
}

/// Asked, and answered with warnings only: the result is the edit's own.
#[test]
fn warnings_alone_add_nothing() {
    let warn = diag(0, 3, Severity::Warning, "function `a` is never used");
    let (src, log) = fake(true, Duration::ZERO, vec![warn]);
    let (got, _, plain) = edit(src, &file("a.rs"));
    assert_eq!(got, plain);
    assert_eq!(*log.lock().unwrap(), ["serves a.rs", "asked a.rs"]);
}

/// A server that takes longer than the bound costs the edit the bound, and
/// the edit says nothing about a file nobody finished looking at.
#[test]
fn a_slow_source_is_given_up_at_the_bound() {
    let (src, _) = fake(true, Duration::from_secs(5), vec![mismatch()]);
    let (got, took, plain) = edit(src, &file("a.rs"));
    assert!(
        took < Duration::from_secs(4),
        "waited out the source: {took:?}"
    );
    assert_eq!(got, plain);
}

#[test]
fn a_failed_edit_asks_nothing() {
    let (src, log) = fake(true, Duration::ZERO, vec![mismatch()]);
    let path = file("a.rs");
    let args = json!({"path": path, "old": "no such text", "new": "x"}).to_string();
    let err = sys_surface_diagnosing(src)
        .call("sys", "edit", &args)
        .unwrap_err();
    assert!(!err.contains("diagnostics now"), "{err}");
    assert!(log.lock().unwrap().is_empty(), "{log:?}");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// A file no installed server serves is never asked about, and one in a
/// language nobody could serve does not even reach the source.
#[test]
fn a_file_nobody_serves_is_never_asked_about() {
    let (src, log) = fake(false, Duration::ZERO, vec![mismatch()]);
    let (got, _, plain) = edit(Arc::clone(&src), &file("a.rs"));
    assert_eq!(got, plain);
    assert_eq!(*log.lock().unwrap(), ["serves a.rs"]);
    let (src, log) = fake(true, Duration::ZERO, vec![mismatch()]);
    let (got, _, plain) = edit(src, &file("notes.md"));
    assert_eq!(got, plain);
    assert!(log.lock().unwrap().is_empty(), "{log:?}");
}

/// A whole-file write is a write too; a read is not.
#[test]
fn a_write_file_is_asked_about_and_a_read_is_not() {
    let (src, log) = fake(true, Duration::ZERO, vec![mismatch()]);
    let path = file("a.rs");
    let t = sys_surface_diagnosing(src);
    let read = t.call("sys", "read_file", &json!({"path": path}).to_string());
    assert!(!read.unwrap().contains("diagnostics now"));
    assert!(log.lock().unwrap().is_empty(), "{log:?}");
    let args = json!({"path": path, "content": "fn a() -> i32 { \"s\" }\n"}).to_string();
    let wrote = t.call("sys", "write_file", &args).unwrap();
    assert!(wrote.ends_with(&format!("\n\n{NOTE}")), "{wrote}");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

// ---- the note itself --------------------------------------------------------

#[test]
fn the_note_lists_five_errors_in_file_order_and_counts_the_rest() {
    let mut list: Vec<Diagnostic> = (0..7)
        .rev()
        .map(|i| diag(i * 10, 0, Severity::Error, &format!("e{i}")))
        .collect();
    list.push(diag(1, 0, Severity::Warning, "w"));
    let note = block(&list).unwrap();
    assert_eq!(
        note,
        "diagnostics now: 7 new errors\n  1:1 error: e0\n  11:1 error: e1\n  21:1 error: e2\n  \
         31:1 error: e3\n  41:1 error: e4\n  \u{2026} 2 more"
    );
}

#[test]
fn a_long_message_is_cut_and_no_errors_is_no_note() {
    let long = "x".repeat(500);
    let note = block(&[diag(0, 0, Severity::Error, &long)]).unwrap();
    let line = note.lines().nth(1).unwrap();
    assert_eq!(line.chars().count(), LINE_CAP + 1, "{line}");
    assert!(line.ends_with('\u{2026}'));
    assert_eq!(block(&[]), None);
    assert_eq!(block(&[diag(0, 0, Severity::Hint, "h")]), None);
}
