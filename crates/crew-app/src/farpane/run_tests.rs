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
