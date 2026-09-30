use super::file::{order, stamp, write, DIR, KEEP};
use super::spilltest::guard;
use super::*;
use serde_json::json;
use std::path::Path;

/// `n` numbered lines, the kind of output a test log is.
fn log(n: usize) -> String {
    (1..=n).map(|i| format!("line {i} of the log\n")).collect()
}

/// The `.txt` files in `root/.crew/out`, sorted.
fn spills(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root.join(DIR)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".txt"))
        .collect();
    names.sort();
    names
}

#[test]
fn the_stamp_is_the_utc_date_and_time() {
    assert_eq!(stamp(0), "19700101-000000");
    assert_eq!(stamp(1_000_000_000), "20010909-014640");
    assert_eq!(stamp(951_782_400), "20000229-000000");
    assert_eq!(stamp(1_790_000_000), "20260921-141320");
}

#[test]
fn the_whole_text_is_written_and_its_path_is_relative() {
    let g = guard("write");
    let text = log(3_000);
    let rel = write(g.dir(), "run", &text).expect("written");
    assert!(rel.starts_with(".crew/out/run-"), "{rel}");
    assert!(order(rel.rsplit('/').next().unwrap()).is_some(), "{rel}");
    // Crew's own file: never one of a task's changed files, never undone.
    assert!(super::super::changed::is_crew_artifact(&rel), "{rel}");
    assert_eq!(std::fs::read_to_string(g.dir().join(&rel)).unwrap(), text);
    // The first spill leaves a .gitignore that ignores everything, itself too.
    let ignore = std::fs::read_to_string(g.dir().join(DIR).join(".gitignore")).unwrap();
    assert_eq!(ignore.lines().last(), Some("*"), "{ignore}");
}

#[test]
fn only_the_newest_thirty_are_kept() {
    let g = guard("prune");
    let rels: Vec<String> = (0..KEEP + 5)
        .map(|i| write(g.dir(), "grep", &format!("spill {i}\n")).unwrap())
        .collect();
    assert_eq!(spills(g.dir()).len(), KEEP);
    for (i, rel) in rels.iter().enumerate() {
        assert_eq!(g.dir().join(rel).exists(), i >= 5, "{rel}");
    }
    // What is not a spill is never pruned.
    assert!(g.dir().join(DIR).join(".gitignore").exists());
}

#[test]
fn a_long_result_is_saved_and_the_pointer_fits_beside_it() {
    let g = guard("saved");
    let text = log(3_000);
    let spill = saved("run", &text);
    let name = spills(g.dir()).pop().expect("a spill");
    let rel = format!("{DIR}/{name}");
    let closed = spill.close("the fitted result".into());
    let lines = grouped(3_000);
    let kb = text.len().div_ceil(1024);
    assert_eq!(
        closed,
        format!(
            "the fitted result\n\u{2026} full output ({lines} lines, {kb} KB) saved to {rel} \
             \u{2014} read it with sys:read_file {{\"path\": \"{rel}\", \"line\": N}} or \
             search it with sys:grep"
        )
    );
    // The room is what is left of the budget after the pointer and its newline.
    let pointer = closed.lines().last().unwrap();
    assert_eq!(spill.room + 1 + pointer.len(), RUN_FIT);
}

#[test]
fn a_short_result_saves_nothing_and_ends_as_it_did() {
    let g = guard("short");
    let spill = saved("run", "exit 0\nhi\n");
    assert!(!spill.is_saved());
    assert_eq!(spill.room, RUN_FIT);
    assert_eq!(spill.close("exit 0\nhi\n".into()), "exit 0\nhi\n");
    assert!(
        !g.dir().join(".crew").exists(),
        "no directory made for nothing"
    );
}

#[test]
fn crew_spill_0_saves_nothing_and_points_nowhere() {
    let g = guard("off");
    std::env::set_var("CREW_SPILL", "0");
    let spill = saved("run", &log(3_000));
    std::env::remove_var("CREW_SPILL");
    assert!(!spill.is_saved());
    assert_eq!(spill.close("x".into()), "x");
    assert!(spills(g.dir()).is_empty());
}

#[test]
fn an_io_error_saves_nothing() {
    let g = guard("io");
    // A file where the directory would go: `create_dir_all` fails.
    std::fs::write(g.dir().join(".crew"), "not a directory").unwrap();
    assert!(write(g.dir(), "run", &log(3_000)).is_none());
    assert!(!saved("run", &log(3_000)).is_saved());
}

#[test]
fn a_spill_past_a_megabyte_is_cut_and_says_so() {
    let g = guard("cap");
    let text = log(80_000);
    assert!(text.len() > CAP);
    assert!(saved("run", &text).is_saved());
    let name = spills(g.dir()).pop().unwrap();
    let body = std::fs::read_to_string(g.dir().join(DIR).join(name)).unwrap();
    assert!(body.len() <= CAP, "{}", body.len());
    assert!(body.ends_with(CUT), "{}", &body[body.len() - 80..]);
}

/// The pointer is only worth writing if both calls it names work on the file
/// — in a work tree whose `.gitignore` names `.crew/`, which crew's own does.
#[test]
fn read_file_and_grep_reach_a_spill_in_an_ignored_dir() {
    let g = guard("reach");
    let ok = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(g.dir())
        .status()
        .unwrap()
        .success();
    assert!(ok);
    std::fs::write(g.dir().join(".gitignore"), ".crew/\n").unwrap();
    let rel = write(g.dir(), "run", &log(3_000)).unwrap();
    let path = g.dir().join(&rel).display().to_string();
    let page = super::super::sysreadline::read(&path, &json!({"line": 2_500})).unwrap();
    assert!(
        page.starts_with("2500\u{2502} line 2500 of the log\n"),
        "{page}"
    );
    let grep =
        |p: &Path| super::super::sysgrep::grep(&json!({"pattern": "^line 2718 ", "path": p}));
    let file = grep(Path::new(&path)).unwrap();
    assert_eq!(file, format!("{path}:2718: line 2718 of the log"));
    let dir = grep(&g.dir().join(DIR)).unwrap();
    assert!(dir.ends_with(":2718: line 2718 of the log"), "{dir}");
}
