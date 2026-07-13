//! `PtyTerm::spawn_with_env` must expose the given variables to the child —
//! the mechanism behind run panes seeing the user's login-shell PATH instead
//! of the minimal one a Dock-launched app inherits.
use std::io::Write;
use std::time::{Duration, Instant};

use crew_term::{GridSize, PtyTerm, TermModel};

#[test]
fn spawn_with_env_reaches_child() {
    let mut term = PtyTerm::spawn_with_env(
        GridSize { cols: 80, rows: 10 },
        "sh",
        &[],
        None,
        &[("CREW_ENV_TEST_MARKER", "crew_env_landed")],
    )
    .unwrap();
    let mut w = term.writer();
    w.write_all(b"echo \"got:$CREW_ENV_TEST_MARKER\"\n")
        .unwrap();
    w.flush().unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut found = false;
    while Instant::now() < deadline {
        term.try_read();
        let line: String = {
            let mut cs: Vec<_> = term.cells(true);
            cs.sort_by_key(|c| (c.row, c.col));
            cs.iter().map(|c| c.c).collect()
        };
        if line.contains("got:crew_env_landed") {
            found = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        found,
        "child should see the env var passed to spawn_with_env"
    );
}
