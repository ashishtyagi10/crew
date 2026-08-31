use super::*;
use std::process::Command;
use std::time::Duration;

/// The child's own children go too. A broker spawns agent CLIs, and
/// killing a parent does not kill its children — measured before the fix,
/// a killed broker left a running `claude` alive and reparented, still
/// working and still spending. `spawn` puts the broker in its own process
/// group so `drop` can take the group.
#[test]
#[cfg(unix)]
fn dropping_the_plugin_kills_the_whole_tree() {
    // A child that spawns a grandchild, records its pid, and waits — so
    // both are alive when the plugin is dropped. The pid goes to a file
    // rather than stdout: the reader thread forwards parsed events only.
    let pidfile = std::env::temp_dir().join(format!(
        "crew-tree-test-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let script = format!(
        "sh -c 'sleep 30' & echo $! > {} ; sleep 30",
        pidfile.display()
    );
    let p = Plugin::spawn("sh", &["-c".to_string(), script]).unwrap();
    let mut grandchild = 0i32;
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(50));
        if let Ok(t) = std::fs::read_to_string(&pidfile) {
            if let Ok(pid) = t.trim().parse() {
                grandchild = pid;
                break;
            }
        }
    }
    assert!(grandchild > 0, "grandchild never reported its pid");
    let alive = |pid: i32| {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    assert!(alive(grandchild), "grandchild should be running");
    drop(p);
    std::thread::sleep(Duration::from_millis(400));
    assert!(
        !alive(grandchild),
        "grandchild {grandchild} outlived the plugin that owned it"
    );
    let _ = std::fs::remove_file(&pidfile);
}

/// Unique temp names, so two of these can run at once.
static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[test]
fn dropping_the_plugin_kills_the_child() {
    // A long-lived child standing in for the broker subprocess.
    let p = Plugin::spawn("sh", &["-c".to_string(), "sleep 30".to_string()]).unwrap();
    let pid = p.child_id();
    drop(p);
    std::thread::sleep(Duration::from_millis(300));
    // `kill -0` succeeds only while the process exists; once killed and reaped
    // it exits non-zero.
    let alive = Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(
        !alive,
        "broker child {pid} should be killed when the Plugin drops"
    );
}
