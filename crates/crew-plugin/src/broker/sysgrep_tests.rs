use super::*;
use std::path::PathBuf;

fn tree(files: &[(&str, &str)]) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("crew-grep-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for (rel, text) in files {
        let p = d.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    d
}

fn args(v: serde_json::Value) -> serde_json::Value {
    v
}

#[test]
fn grep_finds_the_line_and_names_its_file_and_number() {
    let d = tree(&[
        (
            "src/todo/checkbox.rs",
            "const SIDE: f32 = 0.8;\nfn draw() {}\n",
        ),
        ("src/other.rs", "fn nothing() {}\n"),
        ("target/debug/checkbox.rs", "const SIDE: f32 = 9.9;\n"),
        (".git/HEAD", "const SIDE\n"),
    ]);
    let out = grep(&args(
        serde_json::json!({"pattern": "const SIDE", "path": d}),
    ))
    .unwrap();
    assert_eq!(
        out, "src/todo/checkbox.rs:1: const SIDE: f32 = 0.8;",
        "{out}"
    );
}

#[test]
fn grep_filters_by_glob_and_folds_case_on_request() {
    let d = tree(&[("a.rs", "Draw the BOX\n"), ("a.md", "draw the box\n")]);
    let out = grep(&serde_json::json!({"pattern": "draw the box", "path": d, "glob": "*.rs", "ignore_case": true}))
        .unwrap();
    assert_eq!(out, "a.rs:1: Draw the BOX", "{out}");
    let none =
        grep(&serde_json::json!({"pattern": "draw the box", "path": d, "glob": "*.rs"})).unwrap();
    assert!(none.starts_with("no match"), "{none}");
}

#[test]
fn grep_skips_binaries_and_fits_its_output() {
    let many: String = (0..400).map(|i| format!("hit {i}\n")).collect();
    let d = tree(&[("big.txt", &many), ("bin.dat", "hit\0hit\n")]);
    let out = grep(&serde_json::json!({"pattern": "^hit", "path": d})).unwrap();
    assert!(
        out.len() <= crate::broker::toolclip::RUN_FIT,
        "{}",
        out.len()
    );
    assert!(
        out.ends_with(&format!("in 1 file left out \u{2014} {NARROW}")),
        "{out}"
    );
    assert!(!out.contains("bin.dat"), "binary skipped");
}

#[test]
fn a_bad_regex_or_path_is_an_error_the_agent_can_read() {
    assert!(grep(&serde_json::json!({"pattern": "("}))
        .unwrap_err()
        .contains("not a valid regex"));
    assert!(
        grep(&serde_json::json!({"pattern": "x", "path": "/no/such/dir"}))
            .unwrap_err()
            .contains("no such path")
    );
}

#[test]
fn glob_matches_names_and_paths() {
    let d = tree(&[
        ("crates/a/src/mod.rs", ""),
        ("crates/b/src/deep/mod.rs", ""),
        ("crates/a/src/checkbox_tests.rs", ""),
        ("node_modules/x/mod.rs", ""),
    ]);
    let names = glob(&serde_json::json!({"pattern": "*checkbox*", "path": d})).unwrap();
    assert_eq!(names, "crates/a/src/checkbox_tests.rs");
    let paths = glob(&serde_json::json!({"pattern": "crates/**/mod.rs", "path": d})).unwrap();
    assert_eq!(
        paths, "crates/a/src/mod.rs\ncrates/b/src/deep/mod.rs",
        "{paths}"
    );
}

#[test]
fn glob_patterns_mean_what_a_shell_means() {
    assert!(glob_match("*.rs", "src/a.rs"));
    assert!(!glob_match("*.rs", "src/a.rsx"));
    assert!(glob_match("src/**/*.ts", "src/x/y/z.ts"));
    assert!(glob_match("src/**/*.ts", "src/z.ts"));
    assert!(!glob_match("src/*.ts", "src/x/z.ts"));
    assert!(glob_match("a?c.md", "abc.md"));
}

/// Numbered lines, with `HIT n` on the hit lines and line 22 blank.
fn numbered(to: usize, hits: &[usize]) -> String {
    (1..=to)
        .map(|n| match n {
            22 => "\n".to_string(),
            n if hits.contains(&n) => format!("HIT {n}\n"),
            n => format!("line {n}\n"),
        })
        .collect()
}

#[test]
fn context_shows_the_lines_around_one_hit() {
    let d = tree(&[("a.rs", &numbered(10, &[5]))]);
    let out = grep(&serde_json::json!({"pattern": "HIT", "path": d, "context": 2})).unwrap();
    assert_eq!(
        out,
        "a.rs-3- line 3\na.rs-4- line 4\na.rs:5: HIT 5\na.rs-6- line 6\na.rs-7- line 7"
    );
}

/// Hits whose context meets or touches share one group, as `grep -C` prints
/// them; a gap, or another file, opens the next with `--`. Context stops at
/// a file's ends, and a blank context line carries no trailing space.
#[test]
fn nearby_hits_share_a_group_and_groups_are_separated() {
    let d = tree(&[
        ("a.rs", &numbered(30, &[4, 8, 20])),
        ("b.rs", "HIT 1\nline 2\nline 3\nline 4\n"),
    ]);
    let out = grep(&serde_json::json!({"pattern": "HIT", "path": d, "context": 2})).unwrap();
    let want = "a.rs-2- line 2\na.rs-3- line 3\na.rs:4: HIT 4\na.rs-5- line 5\na.rs-6- line 6\n\
                a.rs-7- line 7\na.rs:8: HIT 8\na.rs-9- line 9\na.rs-10- line 10\n--\n\
                a.rs-18- line 18\na.rs-19- line 19\na.rs:20: HIT 20\na.rs-21- line 21\na.rs-22-\n--\n\
                b.rs:1: HIT 1\nb.rs-2- line 2\nb.rs-3- line 3";
    assert_eq!(out, want);
    // [2, 6] and [7, 11] touch: one group, no `--` between.
    let d = tree(&[("c.rs", &numbered(12, &[4, 9]))]);
    let out = grep(&serde_json::json!({"pattern": "HIT", "path": d, "context": 2})).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 10, "lines 2 to 11: {out}");
    assert!(!out.contains("--"), "{out}");
}

#[test]
fn context_is_a_small_count_of_lines() {
    let d = tree(&[("a.rs", &numbered(20, &[10]))]);
    let lines = |c: serde_json::Value| {
        grep(&serde_json::json!({"pattern": "HIT", "path": d, "context": c}))
            .map(|out| out.lines().count())
    };
    assert_eq!(lines(serde_json::json!(0)), Ok(1));
    assert_eq!(lines(serde_json::json!("1")), Ok(3), "a quoted number");
    assert_eq!(lines(serde_json::json!(40)), Ok(11), "at most five a side");
    assert!(lines(serde_json::json!(true))
        .unwrap_err()
        .contains("context"));
}
