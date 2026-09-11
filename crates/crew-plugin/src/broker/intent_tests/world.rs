//! The classifier is shown the room it routes in — roster, tree, tools —
//! each fact only when known, after the invariant grammar and before the
//! message, and a bare world leaves the prompt as it always was.
use super::*;
use crate::broker::intent::classify::prompt;
use crate::broker::intent::world::dirty_count;
use std::path::{Path, PathBuf};

const HEAD: &str = "The world you route in:";

fn world(agents: &[&str], dirty: Option<usize>, tools: &[&str]) -> World {
    World {
        agents: agents.iter().map(|s| s.to_string()).collect(),
        dirty,
        tools: tools.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn an_empty_world_says_nothing_and_leaves_the_prompt_bare() {
    assert_eq!(World::default().section(), "");
    let p = prompt("polish the intro", &World::default());
    assert!(!p.contains(HEAD), "{p}");
    assert!(
        p.ends_with("Nothing else.\n\nMessage: polish the intro"),
        "{p}"
    );
}

#[test]
fn each_fact_appears_only_when_known() {
    let s = world(&["planner", "coder"], None, &[]).section();
    assert_eq!(s, format!("{HEAD}\nagents: planner, coder\n\n"));
    let s = world(&[], Some(0), &[]).section();
    assert_eq!(s, format!("{HEAD}\ntree: clean\n\n"));
    let s = world(&[], None, &["weather: forecasts by place"]).section();
    assert_eq!(s, format!("{HEAD}\ntools: weather: forecasts by place\n\n"));
}

#[test]
fn the_tree_line_counts_files_and_knows_one_from_many() {
    assert!(world(&[], Some(1), &[])
        .section()
        .contains("tree: dirty (1 file)\n"));
    assert!(world(&[], Some(4), &[])
        .section()
        .contains("tree: dirty (4 files)\n"));
}

#[test]
fn every_fact_together_is_one_block_in_a_fixed_order() {
    let s = world(&["a", "b"], Some(2), &["x: one", "y: two"]).section();
    assert_eq!(
        s,
        format!("{HEAD}\nagents: a, b\ntree: dirty (2 files)\ntools: x: one; y: two\n\n")
    );
}

#[test]
fn a_long_tool_surface_is_clipped_to_one_line() {
    let many: Vec<String> = (0..40)
        .map(|i| format!("tool{i}: does thing {i}"))
        .collect();
    let refs: Vec<&str> = many.iter().map(String::as_str).collect();
    let s = world(&[], None, &refs).section();
    let line = s.lines().find(|l| l.starts_with("tools: ")).unwrap();
    assert!(line.chars().count() < 420, "{}", line.chars().count());
    assert!(line.ends_with('\u{2026}'), "{line}");
}

#[test]
fn the_world_sits_after_the_grammar_and_before_the_message() {
    let p = prompt(
        "polish the intro",
        &world(&["planner", "coder"], Some(3), &[]),
    );
    let grammar = p.find("SHAPE: <reply").unwrap();
    let rounds = p.find("ROUNDS: <1-").unwrap();
    let block = p.find(HEAD).unwrap();
    let msg = p.find("Message: polish the intro").unwrap();
    assert!(grammar < rounds && rounds < block && block < msg, "{p}");
    assert!(p.ends_with("Message: polish the intro"), "{p}");
}

#[test]
fn the_classifier_is_handed_the_world_it_routes_in() {
    let seen = std::sync::Mutex::new(String::new());
    let call = |p: &str| {
        *seen.lock().unwrap() = p.to_string();
        Ok("SHAPE: fan\nAGENTS: coder".to_string())
    };
    let w = world(&["planner", "coder"], Some(2), &["lsp (rust): hover"]);
    let r = decide_in("two takes on this", &w, Some(&call));
    let p = seen.lock().unwrap();
    assert!(p.contains("agents: planner, coder"), "{p}");
    assert!(p.contains("tree: dirty (2 files)"), "{p}");
    assert!(p.contains("tools: lsp (rust): hover"), "{p}");
    // …and its roster is the set an AGENTS: line may pick from.
    assert_eq!(r.decision().hints.agents, Some(vec!["coder".to_string()]));
}

fn git(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A throwaway repo with one committed file, in the OS temp dir.
fn repo() -> PathBuf {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let id = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("crew-world-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@t"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join("a.txt"), "one\n").unwrap();
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-qm", "seed"]);
    dir
}

#[test]
fn the_dirty_probe_counts_changed_paths_and_not_crews_own() {
    let dir = repo();
    assert_eq!(dirty_count(&dir), Some(0), "a fresh commit is clean");
    std::fs::write(dir.join("a.txt"), "two\n").unwrap();
    std::fs::write(dir.join("b.txt"), "new\n").unwrap();
    std::fs::create_dir_all(dir.join(".crew")).unwrap();
    std::fs::write(dir.join(".crew").join("session-live.md"), "x").unwrap();
    assert_eq!(
        dirty_count(&dir),
        Some(2),
        "one edit, one new file, no .crew"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn outside_a_repository_the_tree_is_simply_unknown() {
    let dir = std::env::temp_dir().join(format!("crew-world-norepo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // `git status` in a plain directory fails; a missing directory can't run.
    assert_eq!(dirty_count(&dir), None);
    assert_eq!(dirty_count(&dir.join("missing")), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn gather_reads_the_sessions_roster_names_in_order() {
    let _g = testenv::mock_with_specialists("ok", testenv::TRIO);
    let w = World::gather(&Session::new());
    assert_eq!(w.agents, vec!["planner", "coder", "reviewer"]);
}
