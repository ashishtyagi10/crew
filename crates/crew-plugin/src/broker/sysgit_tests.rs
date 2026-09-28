use super::*;
use crate::broker::tier::{sys_tier, Tier};
use serde_json::json;
use std::path::PathBuf;

fn sh(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?}");
}

/// A repository on branch `trunk` with two commits by Ada Lovelace — `a.txt`,
/// then `b.txt` — and `a.txt` changed since.
fn repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-sysgit-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    sh(&dir, &["init", "-q"]);
    sh(&dir, &["symbolic-ref", "HEAD", "refs/heads/trunk"]);
    sh(&dir, &["config", "user.email", "ada@example.com"]);
    sh(&dir, &["config", "user.name", "Ada Lovelace"]);
    sh(&dir, &["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.join("a.txt"), "one\ntwo\n").unwrap();
    sh(&dir, &["add", "-A"]);
    sh(&dir, &["commit", "-q", "-m", "plant the tree"]);
    std::fs::write(dir.join("b.txt"), "b\n").unwrap();
    sh(&dir, &["add", "-A"]);
    sh(&dir, &["commit", "-q", "-m", "second thoughts"]);
    std::fs::write(dir.join("a.txt"), "one\ntwo\nthree\n").unwrap();
    dir
}

fn git(dir: &Path, cmd: &str, args: &[&str]) -> Result<String, String> {
    git_in(dir, &json!({"cmd": cmd, "args": args}))
}

#[test]
fn status_names_the_branch_and_the_modified_file() {
    let d = repo("status");
    let out = git_in(&d, &json!({"cmd": "status"})).unwrap();
    assert!(out.contains("## trunk"), "{out}");
    assert!(out.contains(" M a.txt"), "{out}");
    let _ = std::fs::remove_dir_all(&d);
}

/// `-n 20 --oneline` are defaults, set before the agent's arguments so that
/// its own `-n 1` wins.
#[test]
fn log_is_one_line_a_commit_and_the_agents_count_wins() {
    let d = repo("log");
    let all = git(&d, "log", &[]).unwrap();
    assert_eq!(all.lines().count(), 2, "{all}");
    assert!(
        all.lines().next().unwrap().ends_with(" second thoughts"),
        "{all}"
    );
    let one = git(&d, "log", &["-n", "1"]).unwrap();
    assert!(one.contains("second thoughts"), "{one}");
    assert!(!one.contains("plant the tree"), "{one}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn blame_names_who_wrote_the_line() {
    let d = repo("blame");
    let out = git(&d, "blame", &["-L", "1,1", "a.txt"]).unwrap();
    assert!(out.contains("Ada Lovelace"), "{out}");
    assert_eq!(out.lines().count(), 1, "{out}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn diff_is_the_added_line_under_a_stat() {
    let d = repo("diff");
    let out = git(&d, "diff", &[]).unwrap();
    assert!(out.contains("+three"), "{out}");
    assert!(out.contains("1 file changed"), "{out}");
    // A shape the agent chose is the shape it gets: no patch under --stat.
    let stat = git(&d, "diff", &["--stat"]).unwrap();
    assert!(stat.contains("a.txt"), "{stat}");
    assert!(!stat.contains("+three"), "{stat}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_failing_git_is_an_error_carrying_its_stderr() {
    let d = repo("fail");
    let e = git(&d, "log", &["no-such-revision"]).unwrap_err();
    assert!(e.contains("no-such-revision"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

/// The file `--output` would have written is checked BEFORE the error, so a
/// refusal that let it through fails on the write, not only on the `Ok`.
#[test]
fn an_argument_that_writes_or_runs_is_refused_and_nothing_is_written() {
    let d = repo("refuse");
    let target = d.join("written-by-git.patch");
    let flag = format!("--output={}", target.display());
    for cmd in ["diff", "log", "show"] {
        let r = git(&d, cmd, &["-p", &flag]);
        assert!(!target.exists(), "git {cmd} wrote {}", target.display());
        let e = r.unwrap_err();
        assert!(e.contains("writes the result to a file"), "{cmd}: {e}");
    }
    for (arg, why) in [
        ("--ext-diff", "runs the diff program"),
        ("--textconv", "runs the converter"),
        ("-c", "rewrites git's config"),
        ("--exec=sh", "runs a program"),
        ("--git-dir=/elsewhere", "another repository"),
        ("HEAD\n--output=x", "newline"),
        ("--show-signature", "runs gpg"),
    ] {
        let e = git(&d, "diff", &[arg]).unwrap_err();
        assert!(e.contains("refuses") && e.contains(why), "{arg}: {e}");
    }
    let e = git(&d, "show", &["-o", "x"]).unwrap_err();
    assert!(e.contains("names a file to write"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_subcommand_that_changes_the_repository_is_refused() {
    let d = repo("sub");
    for cmd in ["push", "commit", "checkout", "reset", "config"] {
        let e = git(&d, cmd, &[]).unwrap_err();
        assert!(e.contains("goes through sys:run"), "{cmd}: {e}");
    }
    // Nothing was committed: a.txt is still modified.
    assert!(git(&d, "status", &[]).unwrap().contains(" M a.txt"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_long_log_keeps_its_start_and_counts_the_rest() {
    let log: String = (0..400)
        .map(|i| format!("abc{i:04} commit {i}\n"))
        .collect();
    let out = fitted("log", &log, false);
    assert!(out.len() <= RUN_FIT, "{}", out.len());
    assert!(out.starts_with("abc0000 commit 0\n"), "{out}");
    assert!(out.ends_with("or -L)"), "{out}");
    assert!(out.contains(" more lines "), "{out}");
}

#[test]
fn git_is_a_read() {
    assert_eq!(sys_tier("git"), Some(Tier::Read));
    assert!(!crate::broker::tier::blocked_by_read_only("git"));
}
