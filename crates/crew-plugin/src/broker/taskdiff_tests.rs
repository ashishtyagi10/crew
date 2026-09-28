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

// ---- the two messages ----------------------------------------------------

/// Everything `deliver` emits, in order, for a patch and a verdict.
fn delivered(patch: &str, diag: Diag) -> Vec<String> {
    let mut out = Vec::new();
    deliver(patch, &["x.rs".to_string()], |_| diag, &mut |m| out.push(m));
    out
}

#[test]
fn nothing_changed_is_no_body_at_all() {
    let dir = std::env::temp_dir();
    let mut out = Vec::new();
    report(
        &dir,
        "deadbeef",
        &[],
        |_| panic!("no files, no ask"),
        &mut |m| out.push(m),
    );
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn an_empty_patch_with_no_server_is_nothing() {
    assert!(delivered("", Diag::NoServer).is_empty());
    assert!(delivered("  \n", Diag::NoServer).is_empty());
}

#[test]
fn no_server_means_the_patch_alone() {
    assert_eq!(delivered("+a", Diag::NoServer), vec!["```diff\n+a\n```"]);
}

/// The verdict is its own message, after the diff — not a paragraph the
/// diff has to wait for.
#[test]
fn a_clean_run_says_so_in_one_line_after_the_diff() {
    assert_eq!(
        delivered("+a", Diag::Clean(3)),
        vec![
            "```diff\n+a\n```".to_string(),
            "no diagnostics in the 3 changed files".into(),
        ]
    );
    assert_eq!(
        delivered("+a", Diag::Clean(1))[1],
        "no diagnostics in the changed file"
    );
}

#[test]
fn diagnostics_are_listed_one_per_line() {
    let lines = vec![
        "src/main.rs:12:8 \u{2014} error [rustc]: cannot find value `x`".to_string(),
        "src/lib.rs:1:1 \u{2014} warning [rustc]: unused import".to_string(),
    ];
    let out = delivered("+a", Diag::Lines(lines.clone()));
    assert_eq!(
        out[1],
        format!("diagnostics after the change:\n{}\n{}", lines[0], lines[1])
    );
    assert!(!out[1].contains("no diagnostics"), "{}", out[1]);
}

#[test]
fn a_flood_of_diagnostics_is_capped_and_counted() {
    let lines: Vec<String> = (0..27)
        .map(|i| format!("f.rs:{i}:1 \u{2014} error: e{i}"))
        .collect();
    let said = verdict(Diag::Lines(lines)).unwrap();
    assert!(said.contains("f.rs:19:1"), "{said}");
    assert!(!said.contains("f.rs:20:1"), "{said}");
    assert!(said.ends_with("\n\u{2026} +7 more"), "{said}");
}

/// Diagnostics with nothing else to show — a mode change, say — still stand
/// on their own.
#[test]
fn diagnostics_stand_without_a_patch() {
    let out = delivered("", Diag::Lines(vec!["a.rs:1:1 \u{2014} error: x".into()]));
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(
        out[0].starts_with("diagnostics after the change:\n"),
        "{out:?}"
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
/// absolute paths, and emits the patch and then the verdict.
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

    let (mut asked, mut out) = (Vec::new(), Vec::new());
    report(
        &dir,
        &base,
        &with_deleted,
        |files| {
            asked = files.to_vec();
            Diag::Clean(files.len())
        },
        &mut |m| out.push(m),
    );
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
    let [diff, said] = &out[..] else {
        panic!("{out:?}")
    };
    assert!(diff.starts_with("```diff\n"), "{diff}");
    assert!(diff.contains("diff --git a/b.rs b/b.rs"), "{diff}");
    assert!(diff.contains("+fn a() { x }"), "{diff}");
    assert_eq!(said, "no diagnostics in the 2 changed files");
    let _ = std::fs::remove_dir_all(&dir);
}
