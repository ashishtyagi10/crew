//! In-band resize reports on a real pty: sent after the winsize changes,
//! once per resize, and only while the program has 2048 on.
use super::*;
use crate::model::TermModel;
use crate::GridSize;
use std::io::Write;
use std::time::{Duration, Instant};

/// Raw mode; optionally enables 2048; prints each report it gets with the
/// `TIOCGWINSZ` it reads right then (`ws=cols x rows`).
fn probe(enable: bool) -> String {
    format!(
        r#"
import fcntl, os, struct, sys, termios, tty
tty.setraw(0)
os.write(1, b"ready\r\n")
if {on}: os.write(1, b"\x1b[?2048h")
def ws():
    r, c, x, y = struct.unpack("HHHH", fcntl.ioctl(0, termios.TIOCGWINSZ, b"\0" * 8))
    return f"{{c}}x{{r}},{{x}}x{{y}}"
while True:
    got = b""
    while not got.endswith(b"t") and not got.endswith(b"q"):
        got += os.read(0, 1)
    if got.endswith(b"q"): break
    os.write(1, b"\r\nrep=" + got[1:-1].replace(b"\x1b", b"^") + b" ws=" + ws().encode() + b"\r\n")
"#,
        on = if enable { "True" } else { "False" }
    )
}

fn screen(t: &PtyTerm) -> String {
    let mut cs = t.cells(false);
    cs.sort_by_key(|c| (c.row, c.col));
    cs.iter().map(|c| c.c).collect()
}

fn pump(t: &mut PtyTerm, want: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !screen(t).contains(want) {
        t.try_read();
        std::thread::sleep(Duration::from_millis(20));
    }
    screen(t)
}

fn spawn(enable: bool) -> PtyTerm {
    let size = GridSize { cols: 80, rows: 24 };
    let mut t = PtyTerm::spawn_args(size, "python3", &["-c".into(), probe(enable)]).unwrap();
    t.set_cell_px(9, 18);
    t
}

#[test]
fn enabled_reports_now_and_after_each_resize_with_the_winsize_already_set() {
    let mut t = spawn(true);
    let s = pump(&mut t, "rep=[48;24;80;432;720ws=80x24,720x432");
    assert!(s.contains("rep=[48;24;80;432;720ws=80x24,720x432"), "{s}");
    t.resize(GridSize {
        cols: 100,
        rows: 30,
    });
    let s = pump(&mut t, "rep=[48;30;100;540;900");
    assert!(s.contains("rep=[48;30;100;540;900ws=100x30,900x540"), "{s}");
    // A new cell size changes the pixels: one more report.
    t.set_cell_px(10, 20);
    let s = pump(&mut t, "rep=[48;30;100;600;1000");
    assert!(
        s.contains("rep=[48;30;100;600;1000ws=100x30,1000x600"),
        "{s}"
    );
    // The same size again reports nothing new.
    t.resize(GridSize {
        cols: 100,
        rows: 30,
    });
    t.set_cell_px(10, 20);
    for _ in 0..15 {
        t.try_read();
        std::thread::sleep(Duration::from_millis(20));
    }
    let s = screen(&t);
    assert_eq!(s.matches("rep=").count(), 3, "{s}");
}

#[test]
fn disabled_a_resize_reports_nothing() {
    let mut t = spawn(false);
    pump(&mut t, "ready");
    t.resize(GridSize {
        cols: 100,
        rows: 30,
    });
    t.set_cell_px(10, 20);
    for _ in 0..15 {
        t.try_read();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!screen(&t).contains("rep="), "{}", screen(&t));
}

#[test]
fn a_dead_programs_2048_goes_when_the_shell_is_back() {
    let mut t = PtyTerm::spawn(GridSize { cols: 60, rows: 8 }, "sh").unwrap();
    let mut w = t.writer();
    w.write_all(b"printf '\\033[?2048h'; echo LEFT\n").unwrap();
    w.flush().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && !t.core.inband_enabled() {
        t.try_read();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(t.core.inband_enabled());
    assert!(t.release_leftover_modes());
    assert!(!t.core.inband_enabled());
    assert!(!t.release_leftover_modes(), "nothing left");
}
