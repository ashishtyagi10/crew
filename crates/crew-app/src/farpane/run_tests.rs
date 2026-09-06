use super::*;
// Only `wait` needs it, and `wait` is Unix-only.
#[cfg(unix)]
use std::time::Duration;

// Only the Unix-only `/bin/sh` tests below wait on a command result.
#[cfg(unix)]
fn wait(rx: Receiver<CmdDone>) -> CmdDone {
    rx.recv_timeout(Duration::from_secs(10))
        .expect("command result")
}

// Hard-codes a POSIX shell and Unix paths (`/bin/sh`, `/tmp`, `/`):
// Unix-only by construction, and nothing about it is portable to a
// Windows runner.
#[cfg(unix)]
#[test]
fn reports_exit_code_and_output_tail() {
    let done = wait(start("/bin/sh", "echo one; echo two", Path::new("/tmp")));
    assert_eq!(done.code, Some(0));
    assert_eq!(done.tail, "two");
}

// Hard-codes a POSIX shell and Unix paths (`/bin/sh`, `/tmp`, `/`):
// Unix-only by construction, and nothing about it is portable to a
// Windows runner.
#[cfg(unix)]
#[test]
fn stderr_wins_the_tail_and_failures_report_nonzero() {
    let done = wait(start(
        "/bin/sh",
        "echo out; echo err >&2; exit 3",
        Path::new("/tmp"),
    ));
    assert_eq!(done.code, Some(3));
    assert_eq!(done.tail, "err");
}

// Hard-codes a POSIX shell and Unix paths (`/bin/sh`, `/tmp`, `/`):
// Unix-only by construction, and nothing about it is portable to a
// Windows runner.
#[cfg(unix)]
#[test]
fn runs_in_the_given_directory() {
    let done = wait(start("/bin/sh", "pwd", Path::new("/")));
    assert_eq!(done.tail, "/");
}

#[test]
fn cd_parsing() {
    assert_eq!(cd_target("cd"), Some("~"));
    assert_eq!(cd_target("cd "), Some("~"));
    assert_eq!(cd_target("cd src/app"), Some("src/app"));
    assert_eq!(cd_target("  cd ~/x  "), Some("~/x"));
    assert_eq!(cd_target("cdx"), None);
    assert_eq!(cd_target("ls"), None);
}

/// A pane rooted at a unique tempdir holding `sub/deep` and `My Docs`.
fn pane(key: &str) -> (std::path::PathBuf, FarPane) {
    let base = std::env::temp_dir().join(format!("crew_far_run_{key}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("sub/deep")).unwrap();
    std::fs::create_dir_all(base.join("My Docs")).unwrap();
    let p = FarPane::new(base.clone());
    (base, p)
}

fn cd(p: &mut FarPane, line: &str) -> std::path::PathBuf {
    p.cmdline = line.into();
    assert!(matches!(run_cmdline(p), FarAction::Status(_)));
    p.left.loc.local_path().unwrap()
}

#[test]
fn cd_dot_dot_lands_on_the_parent_itself() {
    let (base, mut p) = pane("dotdot");
    cd(&mut p, "cd sub/deep");
    assert_eq!(
        cd(&mut p, "cd .."),
        base.join("sub"),
        "no `..` left in the path"
    );
    assert_eq!(
        cd(&mut p, "cd ../"),
        base,
        "trailing slash is not part of the location"
    );
    // Backspace (ascend) from there must go UP, which a `base/sub/..`
    // location got wrong — its own parent was `base/sub`.
    assert_eq!(
        p.left.loc.parent().unwrap().local_path().unwrap(),
        base.parent().unwrap()
    );
}

#[test]
fn cd_takes_a_tab_completed_or_quoted_name_with_a_space() {
    let (base, mut p) = pane("spaces");
    assert_eq!(cd(&mut p, "cd My\\ Docs/"), base.join("My Docs"));
    cd(&mut p, "cd ..");
    assert_eq!(cd(&mut p, "cd \"My Docs\""), base.join("My Docs"));
}
