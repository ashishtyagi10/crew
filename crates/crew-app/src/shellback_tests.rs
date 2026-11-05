//! The shell back, on a real pty: a program left SGR mouse reporting on.
use std::io::Write;
use std::time::{Duration, Instant};

use crate::pane::TermPane;
use crew_term::{GridSize, PtyTerm};

#[test]
fn the_shell_back_clears_the_mouse_a_dead_program_left_on() {
    let pty = PtyTerm::spawn(GridSize { cols: 40, rows: 6 }, "sh").unwrap();
    let input = pty.writer();
    let mut t = TermPane {
        pty,
        input,
        cmd: None,
        cmd_since: None,
        tail: Default::default(),
        read_at: 0,
        spans: Default::default(),
        trail: Default::default(),
        images: Default::default(),
    };
    let mut w = t.pty.writer();
    w.write_all(b"printf '\\033[?1000h\\033[?1006h'\n").unwrap();
    w.flush().unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while !t.pty.input_modes().mouse && Instant::now() < until {
        t.pty.try_read();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(t.pty.input_modes().mouse, "the program turned it on");
    t.spans.started("printf".into(), 0, 1);
    t.shell_back(2, 5);
    let m = t.pty.input_modes();
    assert!(!m.mouse && !m.sgr_mouse, "{m:?}");
    // The wheel scrolls crew's history again instead of typing a report.
    assert_eq!(crate::altscroll::wheel_bytes(&m, 3, (1, 1), 0), None);
}
