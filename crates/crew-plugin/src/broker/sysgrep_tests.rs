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
fn grep_skips_binaries_and_caps_its_output() {
    let many: String = (0..400).map(|i| format!("hit {i}\n")).collect();
    let d = tree(&[("big.txt", &many), ("bin.dat", "hit\0hit\n")]);
    let out = grep(&serde_json::json!({"pattern": "^hit", "path": d})).unwrap();
    assert_eq!(out.lines().count(), MAX_HITS + 1, "capped, with a count");
    assert!(
        out.ends_with("more matches \u{2014} narrow the pattern, path or glob"),
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
