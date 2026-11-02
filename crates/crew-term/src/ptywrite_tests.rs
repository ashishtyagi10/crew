use super::super::*;
use std::io::Write;
use std::time::{Duration, Instant};

/// A paste into a program that isn't reading must not hold up the caller:
/// the window's thread writes every key and paste (2026-11-02: a 4 KiB
/// write blocked 3.8 s on this child, a big one for as long as it lived).
#[cfg(unix)]
#[test]
fn a_write_to_a_program_that_is_not_reading_returns_at_once() {
    let size = GridSize { cols: 40, rows: 10 };
    let args = ["-c".to_string(), "stty raw -echo; sleep 3".to_string()];
    let term = PtyTerm::spawn_args(size, "sh", &args).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let t = Instant::now();
    term.writer().write_all(&[b'x'; 65536]).unwrap();
    assert!(
        t.elapsed() < Duration::from_millis(500),
        "{:?}",
        t.elapsed()
    );
}

/// Separate writes from separate handles still arrive in order.
#[test]
fn writes_reach_the_program_in_the_order_they_were_sent() {
    let mut term = PtyTerm::spawn(GridSize { cols: 40, rows: 10 }, "sh").unwrap();
    let (mut a, mut b) = (term.writer(), term.writer());
    for (i, part) in ["printf 'OR", "DE", "RED", "OK\\n'\n"].iter().enumerate() {
        let w = if i % 2 == 0 { &mut a } else { &mut b };
        w.write_all(part.as_bytes()).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = String::new();
    while Instant::now() < deadline && !seen.contains("ORDEREDOK") {
        term.try_read();
        seen = term.cells(true).iter().map(|c| c.c).collect();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(seen.contains("ORDEREDOK"), "{seen}");
}
