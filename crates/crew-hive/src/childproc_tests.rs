//! The guard that matters more than the helper: a source-tree scan.
//!
//! Adding `CREATE_NO_WINDOW` to today's nine call sites fixes today's bug. The
//! tenth `Command::new` someone writes next month is how it comes back — and
//! it comes back *invisibly on Windows only*, which is precisely the class of
//! regression this repo has already paid for once (a Unix-only `use` dropped
//! the platform for twelve releases because nothing on CI compiled it).
//!
//! So this test reads the crates and fails on a spawn that does not route
//! through [`super::no_console_window`], with an explicit, commented allow-list
//! for the ones that genuinely must not.
use std::path::{Path, PathBuf};

/// Spawn sites that deliberately do NOT hide a console, with the reason.
/// Matched as `file:needle` — the needle appears on the `Command::new` line.
const ALLOWED: &[(&str, &str)] = &[
    // The detached GUI relaunch passes DETACHED_PROCESS, which already means
    // "no console" — and the two flags are mutually exclusive on Windows.
    ("detach.rs", "Command::new(exe)"),
    // Linux-only desktop-database refresh; never compiled on Windows.
    ("reglinux.rs", "Command::new(\"update-desktop-database\")"),
    ("reglinux.rs", "Command::new(\"gtk-update-icon-cache\")"),
    // Unix-only process control (`/bin/sh`, `/bin/kill`, `kill`) inside
    // `#[cfg(unix)]` paths — see the modules' own docs.
    ("sysrun.rs", "Command::new(\"/bin/sh\")"),
    ("sysrun.rs", "Command::new(\"/bin/kill\")"),
    ("host.rs", "Command::new(\"kill\")"),
    // macOS-only TEST probe (`#[cfg(all(test, target_os = "macos"))]`) that
    // cross-checks the appearance preference against the `defaults` CLI.
    // Never compiled on Windows, and never in a shipped binary.
    ("osappearance.rs", "Command::new(\"defaults\")"),
    // Same shape, one preference over: the macOS-only TEST probe that
    // cross-checks Reduce Motion against `defaults`. Test-only, macOS-only.
    ("reducemotion.rs", "Command::new(\"defaults\")"),
    // …and the Increase Contrast one. Same shape, same reason.
    ("oscontrast.rs", "Command::new(\"defaults\")"),
];

/// Walk `dir` for `.rs` files, skipping vendored code and build output.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if p.is_dir() {
            if !matches!(name.as_str(), "target" | "vendor" | ".git") {
                rust_files(&p, out);
            }
        } else if name.ends_with(".rs") {
            out.push(p);
        }
    }
}

/// The workspace root, from this crate's manifest dir.
fn workspace_crates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ dir")
        .to_path_buf()
}

#[test]
fn no_console_window_is_applied_at_every_spawn_site() {
    let mut files = Vec::new();
    rust_files(&workspace_crates(), &mut files);
    assert!(
        files.len() > 50,
        "only found {} source files — the walk is not reaching the crates",
        files.len()
    );

    let mut offenders = Vec::new();
    for f in &files {
        let name = f
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        // The helper itself is not a spawn site, and test code never ships —
        // a console window in a test harness bothers nobody. Test code means
        // any `*_tests.rs`, the `e2e_*` integration files, and anything under
        // a `tests/` or `*_tests/` directory.
        let in_test_tree = f.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s == "tests" || s.ends_with("_tests")
        });
        if name.starts_with("childproc")
            || name.contains("_tests")
            || name.starts_with("e2e_")
            || in_test_tree
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        for (n, line) in lines.iter().enumerate() {
            if !line.contains("Command::new(") || line.trim_start().starts_with("//") {
                continue;
            }
            let allowed = ALLOWED
                .iter()
                .any(|(file, needle)| name == *file && line.contains(needle));
            if allowed {
                continue;
            }
            // Two shapes count as covered: the constructor wrapped inline
            // (`no_console_window(&mut Command::new(…))`), or a binding whose
            // very next line applies the helper to it. Only the next line —
            // any further away and the spawn could happen in between.
            let covered = line.contains("no_console_window(")
                || lines
                    .get(n + 1)
                    .is_some_and(|l| l.contains("no_console_window(&mut "));
            if !covered {
                offenders.push(format!("{name}:{} — {}", n + 1, line.trim()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "these spawn sites do not hide the console, so on Windows each one \
         pops a new terminal window when crew runs detached (no console of its \
         own):\n  {}\n\nWrap them in `crew_hive::childproc::no_console_window` \
         or, if a console really is wanted, add them to ALLOWED with a reason.",
        offenders.join("\n  ")
    );
}

/// The helper must be a pure no-op off Windows — it returns the same command
/// so it can wrap a `Command::new` inline without changing anything else.
#[test]
fn the_helper_returns_the_same_command_for_chaining() {
    let mut cmd = std::process::Command::new("echo");
    let p = super::no_console_window(&mut cmd);
    p.arg("chained");
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        ["chained"],
        "the helper must hand back the same Command so callers can chain"
    );
}

/// A typo here would be silent: an unknown creation flag is simply ignored by
/// `CreateProcessW`, so the windows would keep popping with every test green.
#[cfg(windows)]
#[test]
fn the_flag_is_the_documented_create_no_window() {
    assert_eq!(
        super::CREATE_NO_WINDOW,
        0x0800_0000,
        "CREATE_NO_WINDOW is 0x08000000 in processthreadsapi.h; a wrong value \
         is accepted and ignored by CreateProcessW, so nothing would fail but \
         the console windows would come back"
    );
}

/// The flag must not cost us stdio. The broker speaks a JSON-line protocol
/// over the child's pipes and every `git` call reads stdout — if hiding the
/// console broke redirection, crew would lose both. Runs everywhere, but it is
/// Windows CI that makes it mean something.
#[test]
fn a_hidden_child_still_pipes_its_output_back() {
    let mut cmd = if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "echo piped-through"]);
        c
    } else {
        let mut c = std::process::Command::new("echo");
        c.arg("piped-through");
        c
    };
    super::no_console_window(&mut cmd);
    let out = cmd.output().expect("spawn failed outright");
    assert!(out.status.success(), "hidden child exited {:?}", out.status);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("piped-through"),
        "no stdout came back from a hidden child — hiding the console must \
         not detach the pipes"
    );
}
