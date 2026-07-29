use super::*;

#[test]
fn a_new_pane_is_loading_and_shows_no_content_yet() {
    let p = ViewPane::open(std::env::temp_dir().join("whatever.txt"));
    assert!(p.loading(), "the pane opens before the file is read");
}

#[test]
fn poll_swaps_loading_for_ready_and_reports_the_change() {
    let dir = std::env::temp_dir().join(format!("crew-viewpane-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("ready.txt");
    std::fs::write(&f, "content\n").unwrap();

    let mut p = ViewPane::open(f);
    // Spin until the worker lands; poll() is non-blocking by design.
    let mut changed = false;
    for _ in 0..500 {
        if p.poll() {
            changed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(changed, "poll reports the transition exactly once");
    assert!(!p.loading(), "the pane is no longer loading");
    assert!(!p.poll(), "a settled pane reports no further change");
}

#[test]
fn a_failed_load_lands_in_the_pane_not_a_status_line() {
    // The pane is already on screen by the time this fails, so reporting only
    // to a status line the user may never look at would lose the message.
    let mut p = ViewPane::open(std::path::PathBuf::from("/nonexistent/gone.txt"));
    for _ in 0..500 {
        if p.poll() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    match &p.state {
        LoadState::Failed(msg) => assert!(msg.contains("gone.txt"), "names the file: {msg}"),
        _ => panic!("a missing file must settle as Failed"),
    }
}

#[test]
fn reload_returns_the_pane_to_loading() {
    let mut p = ViewPane::open(std::env::temp_dir().join("x.txt"));
    p.state = LoadState::Failed("stale".into());
    p.reload();
    assert!(p.loading(), "reload re-arms the worker");
}

#[test]
fn reload_drops_a_live_search() {
    // A search's hits point at line indexes in the OLD render; keeping it
    // across a reload would leave `n`/`N` walking lines that may no longer
    // contain what they used to, or may not exist at all.
    let mut p = ViewPane::open(std::env::temp_dir().join("x.txt"));
    p.search = Some(crate::viewpane::search::Search::new(
        "needle".into(),
        vec![1, 2],
    ));
    p.reload();
    assert!(p.search.is_none(), "reload must drop a stale search");
}

#[test]
fn reload_keeps_the_scroll_offset() {
    let dir = std::env::temp_dir().join(format!("crew-viewpane-reload-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("scrolled.txt");
    std::fs::write(&f, "line1\nline2\nline3\n").unwrap();

    let mut p = ViewPane::open(f);
    for _ in 0..500 {
        if p.poll() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!p.loading(), "the initial load must have settled");

    p.scroll = 7;
    p.reload();
    assert!(p.loading(), "reload re-arms the worker");

    for _ in 0..500 {
        if p.poll() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!p.loading(), "the reload must have settled");
    assert_eq!(p.scroll, 7, "reload keeps the pane in place");
}

#[test]
fn a_dead_worker_settles_as_failed_instead_of_loading_forever() {
    // No thread, no sleep: drop the sender immediately so try_recv sees
    // Disconnected on the very first poll.
    let (tx, rx) = std::sync::mpsc::channel::<load::LoadDone>();
    drop(tx);

    let mut p = ViewPane::open(std::env::temp_dir().join("whatever.txt"));
    p.state = LoadState::Loading { since_ms: 0, rx };

    assert!(p.poll(), "a dead worker is itself a state change");
    assert!(!p.loading(), "must not stay Loading forever");
    match &p.state {
        LoadState::Failed(msg) => assert!(!msg.is_empty()),
        _ => panic!("a disconnected channel must settle as Failed"),
    }
}
