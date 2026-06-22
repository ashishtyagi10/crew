//! `PtyTerm::spawn_in` must start the child process in the requested directory —
//! the mechanism behind Crew opening new shells in the input-bar's cwd.
use std::io::Write;
use std::time::{Duration, Instant};

use crew_term::{GridSize, PtyTerm, TermModel};

#[test]
fn spawn_in_starts_in_given_directory() {
    let dir = std::env::temp_dir().join("crew_spawn_in_test_dir");
    std::fs::create_dir_all(&dir).unwrap();
    let canon = dir.canonicalize().unwrap();

    let mut term =
        PtyTerm::spawn_in(GridSize { cols: 80, rows: 10 }, "sh", &[], Some(&canon)).unwrap();
    let mut w = term.writer();
    w.write_all(b"pwd\n").unwrap();
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
        if line.contains("crew_spawn_in_test_dir") {
            found = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(found, "pwd should report the directory spawn_in was given");
}
