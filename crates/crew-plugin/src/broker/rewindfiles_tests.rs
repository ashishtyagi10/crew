//! The confirmation's facts: which files, whose, and whether yes still holds.
use super::rewind_tests::{ask_seen, repo, tree};
use super::*;

use crate::PluginEvent;

/// The fields of an ask's answer these tests read.
struct Ans {
    files: u32,
    names: Vec<String>,
    yours: Vec<String>,
    unsure: bool,
    stamp: String,
    pin: u32,
    error: String,
}

fn ans(ev: PluginEvent) -> Ans {
    match ev {
        PluginEvent::Rewind {
            files,
            names,
            yours,
            unsure,
            stamp,
            pin,
            error,
            ..
        } => Ans {
            files,
            names,
            yours,
            unsure,
            stamp,
            pin,
            error,
        },
        other => panic!("not a rewind answer: {other:?}"),
    }
}

fn asked(s: &Session, dir: &Path, task: u64) -> Ans {
    ans(ask_seen(s, dir, 0, (task, false), ""))
}

/// Task 1 began and ended on a tree, with task 2 after it, all known.
fn two_known(dir: &Path) -> Session {
    let s = Session::default();
    {
        let mut log = lock(&s.rewind);
        log.begin(1, 10);
        log.began_on(1, &tree(dir));
    }
    std::fs::write(dir.join("task.txt"), "made by task 1\n").unwrap();
    {
        let mut log = lock(&s.rewind);
        log.ended_on(1, tree(dir));
        log.begin(2, 20);
        log.began_on(2, &tree(dir));
        log.ended_on(2, tree(dir));
    }
    s
}

#[test]
fn the_files_are_named_and_the_ones_edited_outside_crew_come_first() {
    let dir = repo("names");
    let s = two_known(&dir);
    std::fs::write(dir.join("z-notes.md"), "typed by hand\n").unwrap();
    let a = asked(&s, &dir, 1);
    assert_eq!(a.files, 2);
    assert_eq!(a.names, ["z-notes.md", "task.txt"], "yours first");
    assert_eq!(a.yours, ["z-notes.md"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_long_list_names_five_and_counts_all_with_yours_never_hidden() {
    let dir = repo("many");
    let s = two_known(&dir);
    for n in 0..7 {
        std::fs::write(dir.join(format!("f{n}.txt")), "x\n").unwrap();
    }
    std::fs::write(dir.join("zzz-mine.txt"), "typed\n").unwrap();
    let a = asked(&s, &dir, 1);
    assert_eq!(a.files, 9);
    assert_eq!(a.names.len(), 5);
    assert_eq!(
        a.yours.len(),
        5,
        "eight were typed by hand: all five shown are"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn uncertainty_is_never_no_outside_edits() {
    let dir = repo("unsure");
    // A task that never finished: nothing says what it left.
    let s = Session::default();
    {
        let mut log = lock(&s.rewind);
        log.begin(1, 10);
        log.began_on(1, &tree(&dir));
    }
    assert!(asked(&s, &dir, 1).unsure, "no end recorded");
    // The first task: no earlier tree to compare against.
    lock(&s.rewind).ended_on(1, tree(&dir));
    assert!(asked(&s, &dir, 1).unsure, "first task has no `left`");
    // Fully known, later task: sure.
    let known = two_known(&dir);
    assert!(!asked(&known, &dir, 2).unsure);
    // The last task's end went unrecorded: what came after it is unknown.
    lock(&known.rewind).points[1].post = None;
    assert!(asked(&known, &dir, 2).unsure, "no end for the last task");
    // A git error while comparing: a tree that is not there.
    let bogus = "0123456789012345678901234567890123456789";
    lock(&known.rewind).points[1].post = Some(bogus.into());
    assert!(asked(&known, &dir, 2).unsure, "git failed on the last gap");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_confirm_for_a_list_that_changed_is_refused_and_moves_nothing() {
    let dir = repo("stale");
    let s = two_known(&dir);
    let a = asked(&s, &dir, 1);
    // Same COUNT, different file: a swap an N-files check would pass.
    std::fs::remove_file(dir.join("task.txt")).unwrap();
    std::fs::write(dir.join("other.txt"), "autosaved\n").unwrap();
    let r = ans(ask_seen(&s, &dir, 0, (1, true), &a.stamp));
    assert!(r.error.contains("changed since you looked"), "{}", r.error);
    assert!(
        dir.join("other.txt").exists(),
        "a refused rewind moved files"
    );
    // Edited again, same names: the contents are in the stamp too.
    let b = asked(&s, &dir, 1);
    std::fs::write(dir.join("other.txt"), "autosaved again\n").unwrap();
    let r = ans(ask_seen(&s, &dir, 0, (1, true), &b.stamp));
    assert!(r.error.contains("changed since"), "{}", r.error);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_confirm_for_the_list_shown_goes_and_names_the_pin() {
    let dir = repo("pin");
    let s = two_known(&dir);
    let a = asked(&s, &dir, 1);
    let r = ans(ask_seen(&s, &dir, 0, (1, true), &a.stamp));
    assert!(r.error.is_empty(), "{}", r.error);
    assert!(!dir.join("task.txt").exists());
    assert_eq!(r.pin, 1, "the pin is the newest checkpoint: `undo #1`");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn crew_own_churn_does_not_refuse_a_confirm() {
    let dir = repo("churn");
    let s = two_known(&dir);
    let a = asked(&s, &dir, 1);
    std::fs::create_dir_all(dir.join(".crew")).unwrap();
    std::fs::write(dir.join(".crew/session-live.md"), "streaming\n").unwrap();
    let r = ans(ask_seen(&s, &dir, 0, (1, true), &a.stamp));
    assert!(r.error.is_empty(), "{}", r.error);
    let _ = std::fs::remove_dir_all(&dir);
}
