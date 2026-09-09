use super::*;

// ---- clipping -----------------------------------------------------------

#[test]
fn a_patch_that_fits_is_untouched() {
    let p = "diff --git a/x b/x\n+one\n+two";
    assert_eq!(clip(p, 100, ""), p);
}

/// The cut lands between lines, never inside one: a half line of a diff is a
/// line that lies about what changed.
#[test]
fn a_long_patch_is_cut_on_a_line_boundary_and_the_note_counts_the_rest() {
    let lines: Vec<String> = (0..50)
        .map(|i| format!("+line {i:02} xxxxxxxxxxx"))
        .collect();
    let p = lines.join("\n"); // 50 lines of 20 chars = 1049 chars
    let out = clip(&p, 200, " \u{2014} /diff shows the whole patch");
    let (kept, note) = out.rsplit_once('\n').unwrap();
    // 9 lines of 20 chars + 8 separators = 188 ≤ 200; a tenth would be 209.
    assert_eq!(kept.lines().count(), 9, "{out}");
    assert!(
        kept.lines()
            .all(|l| l.starts_with("+line ") && l.ends_with('x')),
        "a line was cut in half: {kept}"
    );
    assert_eq!(
        note,
        "\u{2026} (+41 more lines \u{2014} /diff shows the whole patch)"
    );
}

/// `/diff` uses the same clip with no "where to read the rest" tail.
#[test]
fn the_hint_is_the_callers() {
    let p = (0..10)
        .map(|_| "+abcdefghij")
        .collect::<Vec<_>>()
        .join("\n");
    let out = clip(&p, 25, "");
    assert!(out.ends_with("\u{2026} (+8 more lines)"), "{out}");
}

/// One line wider than the whole cap — a minified file — still shows its head
/// rather than nothing at all.
#[test]
fn a_single_line_wider_than_the_cap_keeps_its_head() {
    let p = format!("+{}\n+short", "m".repeat(500));
    let out = clip(&p, 50, "");
    assert!(out.starts_with(&format!("+{}", "m".repeat(49))), "{out}");
    assert!(out.ends_with("\u{2026} (+1 more lines)"), "{out}");
}

// ---- fencing ------------------------------------------------------------

#[test]
fn a_plain_patch_gets_the_diff_fence() {
    assert_eq!(fenced("+a\n-b"), "```diff\n+a\n-b\n```");
}

/// A patch that touched a markdown file carries fence lines of its own, and
/// one that is exactly ``` would close the card's fence early and render the
/// rest of the diff as prose.
#[test]
fn a_patch_containing_a_fence_line_gets_a_longer_fence() {
    let p = "+```\n+code\n+```";
    let out = fenced(p);
    assert!(out.starts_with("````diff\n"), "{out}");
    assert!(out.ends_with("\n````"), "{out}");
    // A CONTEXT line is indented one space, and CommonMark closes a fence on
    // up to three spaces of indent — so it counts too.
    let out = fenced(" ```\n+x");
    assert!(out.starts_with("````diff\n"), "{out}");
}

// ---- the body -----------------------------------------------------------

#[test]
fn nothing_changed_is_no_body_at_all() {
    let dir = std::env::temp_dir();
    assert_eq!(
        report(&dir, "deadbeef", &[], |_| panic!("no files, no ask")),
        None
    );
}

#[test]
fn an_empty_patch_with_no_server_is_nothing() {
    assert_eq!(compose("", Diag::NoServer), None);
    assert_eq!(compose("  \n", Diag::NoServer), None);
}

#[test]
fn no_server_means_the_patch_alone() {
    let body = compose("+a", Diag::NoServer).unwrap();
    assert_eq!(body, "```diff\n+a\n```");
    assert!(!body.contains("diagnostics"), "{body}");
}

#[test]
fn a_clean_run_says_so_in_one_line() {
    let body = compose("+a", Diag::Clean(3)).unwrap();
    assert!(
        body.ends_with("\n\nno diagnostics in the 3 changed files"),
        "{body}"
    );
    let one = compose("+a", Diag::Clean(1)).unwrap();
    assert!(
        one.ends_with("\n\nno diagnostics in the changed file"),
        "{one}"
    );
}

#[test]
fn diagnostics_are_listed_after_the_patch() {
    let lines = vec![
        "src/main.rs:12:8 \u{2014} error [rustc]: cannot find value `x`".to_string(),
        "src/lib.rs:1:1 \u{2014} warning [rustc]: unused import".to_string(),
    ];
    let body = compose("+a", Diag::Lines(lines.clone())).unwrap();
    assert_eq!(
        body,
        format!(
            "```diff\n+a\n```\n\ndiagnostics after the change:\n{}\n{}",
            lines[0], lines[1]
        )
    );
    assert!(!body.contains("no diagnostics"), "{body}");
}

#[test]
fn a_flood_of_diagnostics_is_capped_and_counted() {
    let lines: Vec<String> = (0..27)
        .map(|i| format!("f.rs:{i}:1 \u{2014} error: e{i}"))
        .collect();
    let body = compose("+a", Diag::Lines(lines)).unwrap();
    assert!(body.contains("f.rs:19:1"), "{body}");
    assert!(!body.contains("f.rs:20:1"), "{body}");
    assert!(body.ends_with("\n\u{2026} +7 more"), "{body}");
}

/// Diagnostics with nothing else to show — a mode change, say — still stand
/// on their own.
#[test]
fn diagnostics_stand_without_a_patch() {
    let body = compose("", Diag::Lines(vec!["a.rs:1:1 \u{2014} error: x".into()])).unwrap();
    assert!(
        body.starts_with("diagnostics after the change:\n"),
        "{body}"
    );
}

// ---- the real thing, once -------------------------------------------------

fn run(dir: &Path, args: &[&str]) {
    assert!(std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

fn temp_repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-taskdiff-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "t@t"],
        &["config", "user.name", "t"],
    ] {
        run(&dir, args);
    }
    std::fs::write(dir.join("a.rs"), "fn a() {}\n").unwrap();
    run(&dir, &["add", "-A"]);
    run(&dir, &["commit", "-q", "-m", "init"]);
    dir
}

/// `report` hands the diagnostics closure the files that still exist, as
/// absolute paths, and puts the patch in the body.
#[test]
fn report_asks_about_the_surviving_files_and_shows_the_patch() {
    let dir = temp_repo("report");
    let base = crate::broker::checkpoint::worktree_tree(&dir).unwrap();
    std::fs::write(dir.join("a.rs"), "fn a() { x }\n").unwrap();
    std::fs::write(dir.join("b.rs"), "fn b() {}\n").unwrap();
    let changes = super::super::changed::since(&dir, &base).unwrap();
    // Pretend gone.rs was deleted too: it must not be asked about.
    let mut with_deleted = changes.clone();
    with_deleted.push(('D', "gone.rs".into()));

    let mut asked = Vec::new();
    let body = report(&dir, &base, &with_deleted, |files| {
        asked = files.to_vec();
        Diag::Clean(files.len())
    })
    .unwrap();
    let mut names: Vec<String> = asked
        .iter()
        .map(|f| {
            Path::new(f)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names, vec!["a.rs", "b.rs"], "asked: {asked:?}");
    assert!(
        asked.iter().all(|f| Path::new(f).is_absolute()),
        "paths must be absolute: {asked:?}"
    );
    assert!(body.starts_with("```diff\n"), "{body}");
    assert!(body.contains("diff --git a/b.rs b/b.rs"), "{body}");
    assert!(body.contains("+fn a() { x }"), "{body}");
    assert!(
        body.ends_with("no diagnostics in the 2 changed files"),
        "{body}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
