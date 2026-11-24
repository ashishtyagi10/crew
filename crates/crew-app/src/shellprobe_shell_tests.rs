//! The bounded probes against real scripts: a quick one is captured, a hung
//! one is killed near its bound.
use super::*;

/// A throwaway executable shell script at `d/name`, `chmod +x`'d.
fn script(d: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = d.join(name);
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    p
}

/// `probe`, asked again while it comes back empty. Exec'ing a script just
/// written can fail with ETXTBSY ("Text file busy") when another test
/// thread forks while the write fd is open (the child holds a copy until it
/// execs), and the probe reads a failed spawn as no output. That race is the
/// test harness's, not the probe's: a few short retries outlast it.
#[cfg(unix)]
fn settled(probe: impl Fn() -> Option<String>) -> Option<String> {
    (0..5).find_map(|n| {
        if n > 0 {
            std::thread::sleep(Duration::from_millis(100));
        }
        probe()
    })
}

// Writes and executes a `#!/bin/sh` script: Unix-only by construction.
#[cfg(unix)]
#[test]
fn bounded_shell_env_captures_a_quick_probe() {
    let dir = tempfile::tempdir().unwrap();
    let sh = script(dir.path(), "sh", "printf 'ANTHROPIC_API_KEY=sk-ant-1\\n'");
    let out = settled(|| bounded_shell_env(sh.to_str().unwrap(), Duration::from_secs(3)))
        .expect("a fast script must produce output before the deadline");
    assert!(out.contains("ANTHROPIC_API_KEY=sk-ant-1"));
}

#[test]
fn bounded_shell_env_kills_a_hanging_probe_within_the_timeout() {
    // A blocking rc file (stray prompt, hung network mount) must not park
    // this thread for the process lifetime — `Command::output()` alone has
    // no deadline and would wait on this forever.
    let dir = tempfile::tempdir().unwrap();
    let sh = script(dir.path(), "sh", "sleep 5");
    let start = Instant::now();
    let out = bounded_shell_env(sh.to_str().unwrap(), Duration::from_millis(150));
    let elapsed = start.elapsed();
    assert!(out.is_none(), "a hung shell must produce no env to merge");
    assert!(
        elapsed < Duration::from_secs(2),
        "must be killed near the 150ms bound, not wait out the 5s sleep: {elapsed:?}"
    );
}

// Writes and executes a `#!/bin/sh` script: Unix-only by construction.
#[cfg(unix)]
#[test]
fn bounded_shell_path_captures_a_quick_probe() {
    let dir = tempfile::tempdir().unwrap();
    let sh = script(dir.path(), "sh", "printf '/usr/local/bin:/usr/bin:/bin'");
    let out = settled(|| bounded_shell_path(sh.to_str().unwrap(), Duration::from_secs(3)))
        .expect("a fast script must produce output before the deadline");
    assert_eq!(out, "/usr/local/bin:/usr/bin:/bin");
}
