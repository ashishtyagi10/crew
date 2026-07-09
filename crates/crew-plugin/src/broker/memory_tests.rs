use super::*;
use std::path::PathBuf;

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crew-memtest-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn remember_creates_the_file_and_appends() {
    let base = scratch("append");
    let m1 = remember_at(&base, "always use pnpm");
    assert!(m1.contains("remembered"), "confirmation: {m1}");
    let m2 = remember_at(&base, "tests run with --workspace");
    assert!(m2.contains('2'), "counts the notes: {m2}");
    let text = std::fs::read_to_string(base.join(".crew/memory.md")).unwrap();
    assert_eq!(text, "- always use pnpm\n- tests run with --workspace\n");
}

#[test]
fn remember_rejects_an_empty_note() {
    let base = scratch("empty");
    let m = remember_at(&base, "   ");
    assert!(m.contains("usage"), "got: {m}");
    assert!(!base.join(".crew/memory.md").exists());
}

#[test]
fn load_merges_user_then_project_and_caps() {
    let base = scratch("load");
    let user = base.join("user-memory.md");
    let project = base.join("memory.md");
    std::fs::write(&user, "- user rule\n").unwrap();
    std::fs::write(&project, "- project rule\n").unwrap();
    let m = load_from(Some(&user), &project).unwrap();
    let (u, p) = (
        m.find("user rule").unwrap(),
        m.find("project rule").unwrap(),
    );
    assert!(u < p, "user memory comes first");
    // a huge file is clipped with a marker
    std::fs::write(&project, "x".repeat(10_000)).unwrap();
    let m = load_from(Some(&user), &project).unwrap();
    assert!(m.len() <= MEM_CAP + 40, "clipped: {} chars", m.len());
    assert!(m.contains("clipped"));
}

#[test]
fn load_is_none_when_nothing_exists() {
    let base = scratch("none");
    assert_eq!(
        load_from(Some(&base.join("nope.md")), &base.join("also-nope.md")),
        None
    );
    let blank = base.join("blank.md");
    std::fs::write(&blank, "  \n").unwrap();
    assert_eq!(load_from(None, &blank), None);
}

#[test]
fn prepend_wraps_the_task_only_when_memory_exists() {
    assert_eq!(prepend(None, "fix the bug"), "fix the bug");
    let p = prepend(Some("- use tabs".into()), "fix the bug");
    assert!(p.contains("- use tabs") && p.contains("fix the bug"));
    assert!(
        p.to_uppercase().contains("MEMORY"),
        "the block is labeled: {p}"
    );
    let mem = p.find("use tabs").unwrap();
    let task = p.find("fix the bug").unwrap();
    assert!(mem < task, "memory precedes the task");
}
