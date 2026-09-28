//! Through the real tool surface, which decides what a read and a write are,
//! whether they happened, and which task's set they land in.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::broker::session::sessiontest::sys_surface;
use crate::broker::session::Session;
use crew_hive::tools::Tools;

fn dir() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("crew-readset-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn call(t: &dyn Tools, tool: &str, args: Value) -> Result<String, String> {
    t.call("sys", tool, &args.to_string())
}

fn write(t: &dyn Tools, p: &Path, content: &str) -> Result<String, String> {
    call(t, "write_file", json!({"path": p, "content": content}))
}

fn refusal(p: &Path) -> String {
    format!(
        "{} exists and has not been read this task \u{2014} read it first (sys:read_file), or change part of it with sys:edit",
        p.display()
    )
}

/// A task's surface, as a worker builds it from its own snapshot.
fn task(session: &Session) -> Session {
    session.snapshot_with_cancel(Arc::new(AtomicBool::new(false)))
}

#[test]
fn an_existing_file_nobody_read_is_not_overwritten() {
    let d = dir();
    let a = d.join("a.rs");
    std::fs::write(&a, "fn keep() {}\n").unwrap();
    let t = sys_surface();
    assert_eq!(
        write(&*t, &a, "fn guessed() {}\n").unwrap_err(),
        refusal(&a)
    );
    assert_eq!(std::fs::read(&a).unwrap(), b"fn keep() {}\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// Any page will do, and the file is the file however the read spelled it.
#[test]
fn a_read_of_any_page_lets_the_file_be_written() {
    let d = dir();
    let (a, b) = (d.join("a.rs"), d.join("b.rs"));
    std::fs::write(&a, "fn a() {}\nfn b() {}\n").unwrap();
    std::fs::write(&b, "old\n").unwrap();
    let t = sys_surface();
    call(&*t, "read_file", json!({"path": a, "offset": 10})).unwrap();
    write(&*t, &a, "fn c() {}\n").unwrap();
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "fn c() {}\n");
    call(&*t, "read_file", json!({"path": d.join(".").join("b.rs")})).unwrap();
    write(&*t, &b, "new\n").unwrap();
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_read_by_line_counts_as_a_read() {
    let d = dir();
    let a = d.join("a.rs");
    std::fs::write(&a, "one\ntwo\nthree\n").unwrap();
    let t = sys_surface();
    call(&*t, "read_file", json!({"path": a, "line": 2})).unwrap();
    write(&*t, &a, "four\n").unwrap();
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "four\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// A file that was not there is written freely, and one this task wrote it
/// knows whole, so writing it again needs no read in between.
#[test]
fn a_file_this_task_wrote_may_be_written_again() {
    let d = dir();
    let a = d.join("new.rs");
    let t = sys_surface();
    write(&*t, &a, "first\n").unwrap();
    write(&*t, &a, "second\n").unwrap();
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "second\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// The set is the task's: every surface built from one snapshot shares it,
/// and the next task's snapshot starts with nothing read.
#[test]
fn a_new_task_starts_with_nothing_read() {
    let d = dir();
    let a = d.join("a.rs");
    std::fs::write(&a, "kept\n").unwrap();
    let session = Session::default();
    let first = task(&session);
    let reader = first.tools_with_sys(true).unwrap();
    call(&*reader, "read_file", json!({"path": a})).unwrap();
    let writer = first.tools_with_sys(true).unwrap();
    write(&*writer, &a, "by the first task\n").unwrap();
    let next = task(&session).tools_with_sys(true).unwrap();
    assert_eq!(write(&*next, &a, "blind\n").unwrap_err(), refusal(&a));
    assert_eq!(std::fs::read(&a).unwrap(), b"by the first task\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// The broker's real working directory decides "outside": the test's temp
/// directory is not in it, so a missing directory there is not made.
#[test]
fn no_directory_is_made_for_a_path_outside_the_working_directory() {
    let base = dir().join("missing");
    let p = base.join("new").join("a.rs");
    let e = write(&*sys_surface(), &p, "x").unwrap_err();
    assert!(e.contains("only inside the working directory"), "{e}");
    assert!(!base.exists(), "a directory was made outside");
    let _ = std::fs::remove_dir_all(base.parent().unwrap());
}
