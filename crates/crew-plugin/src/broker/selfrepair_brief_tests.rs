//! What the repair pass is handed: the part of the failure that names it,
//! and the change that caused it. Real-shaped output, since the defect this
//! guards against only shows on output shaped like a real build.
use super::*;

use std::path::PathBuf;

const BUILD: &str = include_str!("testdata/check_cargo_build.txt");
const TEST: &str = include_str!("testdata/check_cargo_test.txt");

fn brief<'a>(o: &'a Outcome, passed_before: bool, dir: Option<&'a Path>) -> Failure<'a> {
    Failure {
        cmd: "cargo test",
        o,
        seen: None,
        passed_before,
        dir,
    }
}

fn prompt_for(text: &str) -> String {
    let o = outcome(Ok(text.to_string()));
    repair_task(&brief(&o, false, None), None)
}

#[test]
fn a_build_failure_hands_over_the_error_not_the_compiling() {
    let task = prompt_for(BUILD);
    assert!(task.contains("error[E0308]: mismatched types"), "{task}");
    assert!(task.contains(" --> src/lib.rs:12:5"), "{task}");
    let compiling = task.lines().filter(|l| l.contains("Compiling")).count();
    assert!(compiling <= 3, "{compiling} Compiling lines: {task}");
}

#[test]
fn a_test_failure_hands_over_the_panic_its_values_and_the_summary() {
    let task = prompt_for(TEST);
    assert!(
        task.contains("thread 'broker::y' panicked at src/a.rs:9:5:"),
        "{task}"
    );
    assert!(task.contains("  left: 1"), "{task}");
    assert!(task.contains(" right: 2"), "{task}");
    assert!(
        task.contains("test result: FAILED. 39 passed; 1 failed"),
        "{task}"
    );
}

#[test]
fn output_that_names_no_failure_hands_over_its_end() {
    let steps: String = (0..100).map(|i| format!("step {i} done\n")).collect();
    let task = prompt_for(&format!("exit 1\n{steps}"));
    assert!(task.contains("step 99 done"), "{task}");
    assert!(
        !task.contains("step 0 done"),
        "the head, not the end: {task}"
    );
}

#[test]
fn the_pane_names_the_failing_test_and_the_summary_within_six_lines() {
    let said = line("cargo test", &outcome(Ok(TEST.to_string())));
    let shown: Vec<&str> = said.lines().skip(1).collect();
    assert!(shown.len() <= 6, "{said}");
    assert!(shown.contains(&"test broker::y ... FAILED"), "{said}");
    assert!(
        shown.iter().any(|l| l.starts_with("test result: FAILED.")),
        "{said}"
    );
}

fn git(dir: &Path, args: &[&str]) {
    let ok = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .is_ok_and(|s| s.success());
    assert!(ok, "git {args:?}");
}

fn temp_repo() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-repairbrief-{}-{}",
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
    std::fs::write(dir.join("lib.rs"), "pub fn answer() -> i32 {\n    42\n}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "init"]);
    dir
}

/// The whole path a real failure takes: the tree pinned before the task
/// (`Session::last_tree`), the task's edit, and the pass that is asked.
fn asked_after_edit(passed_before: bool) -> String {
    let dir = temp_repo();
    let session = Session::default();
    let base = crate::broker::checkpoint::worktree_tree(&dir).unwrap();
    *session.last_tree.lock().unwrap() = Some(base);
    std::fs::write(
        dir.join("lib.rs"),
        "pub fn answer() -> i32 {\n    \"forty-two\"\n}\n",
    )
    .unwrap();
    let o = outcome(Ok(BUILD.to_string()));
    let mut asked = String::new();
    let mut repair = |t: &str| {
        asked = t.to_string();
        Ok(())
    };
    let mut emit = |_: PluginEvent| Ok(());
    let f = Failure {
        cmd: "false",
        ..brief(&o, passed_before, Some(&dir))
    };
    take_one_pass(&session, &f, &mut repair, &mut emit).unwrap();
    std::fs::remove_dir_all(&dir).ok();
    asked
}

#[test]
fn the_pass_is_handed_the_change_that_broke_the_check() {
    let asked = asked_after_edit(false);
    assert!(asked.contains("-    42"), "{asked}");
    assert!(asked.contains("+    \"forty-two\""), "{asked}");
    assert!(asked.contains("```diff\n"), "{asked}");
    assert!(
        asked.contains("This is the change the task made:"),
        "{asked}"
    );
    assert!(!asked.contains("passed before"), "{asked}");
    assert!(asked.contains("do not weaken or delete"), "{asked}");
}

#[test]
fn a_check_that_passed_last_time_is_said_to_have_passed_before_the_change() {
    let asked = asked_after_edit(true);
    assert!(
        asked.contains("The check passed before this change:"),
        "{asked}"
    );
    assert!(asked.contains("+    \"forty-two\""), "{asked}");
}

#[test]
fn no_pinned_tree_or_no_edit_means_no_change_is_claimed() {
    let dir = temp_repo();
    let session = Session::default();
    assert_eq!(change_of(&dir, &session), None, "no checkpoint was taken");
    let now = crate::broker::checkpoint::worktree_tree(&dir).unwrap();
    *session.last_tree.lock().unwrap() = Some(now);
    assert_eq!(change_of(&dir, &session), None, "the tree did not move");
    std::fs::remove_dir_all(&dir).ok();
    let o = outcome(Ok(BUILD.to_string()));
    let task = repair_task(&brief(&o, true, None), None);
    assert!(!task.contains("```diff"), "{task}");
    assert!(!task.contains("passed before"), "{task}");
}
