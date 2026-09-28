use super::*;
use std::time::Instant;

#[test]
fn an_open_gate_does_not_wait() {
    let t = Instant::now();
    CkptGate::open_now().wait();
    assert!(t.elapsed() < Duration::from_millis(50));
}

#[test]
fn a_pending_gate_holds_until_the_checkpoint_lands() {
    let g = CkptGate::pending();
    let g2 = Arc::clone(&g);
    let t = Instant::now();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(120));
        g2.open();
    });
    g.wait();
    let waited = t.elapsed();
    assert!(waited >= Duration::from_millis(100), "{waited:?}");
    assert!(waited < WAIT_MAX, "{waited:?}");
    // …and stays open for every later call.
    let t = Instant::now();
    g.wait();
    assert!(t.elapsed() < Duration::from_millis(50));
}

/// The one place a file can change waits for the snapshot: a tool call made
/// while the task's checkpoint is still being taken returns only after it.
#[test]
fn a_tool_call_waits_for_the_snapshot() {
    let mut s = crate::broker::session::Session::default();
    s.ckpt = CkptGate::pending();
    let tools = s.tools_with_sys(true).expect("sys tools");
    let g = Arc::clone(&s.ckpt);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        g.open();
    });
    let t = Instant::now();
    let _ = tools.call("sys", "list_dir", r#"{"path": "."}"#);
    assert!(
        t.elapsed() >= Duration::from_millis(120),
        "{:?}",
        t.elapsed()
    );
}
