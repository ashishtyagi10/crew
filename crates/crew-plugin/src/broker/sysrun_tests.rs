use std::time::{Duration, Instant};

use super::*;

const T: Duration = Duration::from_secs(10);

#[test]
fn run_reports_exit_and_stdout() {
    let r = run_with("echo hi", T).unwrap();
    assert_eq!(r, "exit 0\nhi\n");
}

#[test]
fn run_reports_nonzero_exit_and_stderr_as_ok() {
    let r = run_with("echo oops >&2; exit 3", T).unwrap();
    assert!(r.starts_with("exit 3\n"), "{r}");
    assert!(r.contains("--- stderr ---\noops"), "{r}");
}

#[test]
fn run_times_out_and_kills() {
    let start = Instant::now();
    let e = run_with("sleep 30", Duration::from_millis(200)).unwrap_err();
    assert!(start.elapsed() < Duration::from_secs(5), "killed promptly");
    assert!(e.contains("timed out"), "{e}");
}

#[test]
fn run_caps_output() {
    // ~1 MB of output; capture must stop at CAP without hanging on the pipe.
    let r = run_with("yes x | head -c 1000000", T).unwrap();
    assert!(
        r.len() <= super::super::systools::CAP + 200,
        "got {}",
        r.len()
    );
    assert!(
        r.contains("(output truncated at 64 KB)"),
        "cap notice present"
    );
}

#[test]
fn run_is_noninteractive() {
    // `cat` with a null stdin exits immediately instead of waiting for input.
    let r = run_with("cat", T).unwrap();
    assert_eq!(r, "exit 0\n");
}
