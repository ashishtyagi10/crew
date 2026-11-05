//! A real program on a real pty learns the pane's pixel size: from the
//! pty's own size (`TIOCGWINSZ`) and from `CSI 16 t`, answered on the pty.
use super::*;
use std::io::Write;
use std::time::{Duration, Instant};

/// Waits for a line (so the size is set first), prints `ws=<cols>x<rows>,<xpixel>x<ypixel>`, then asks for the cell
/// size in raw mode and prints the reply with ESC spelled `^`.
const PROBE: &str = r#"
import fcntl, os, struct, sys, termios, tty
sys.stdin.readline()
r, c, x, y = struct.unpack("HHHH", fcntl.ioctl(0, termios.TIOCGWINSZ, b"\0" * 8))
print(f"ws={c}x{r},{x}x{y}", flush=True)
tty.setraw(0)
os.write(1, b"\x1b[16t")
got = b""
while not got.endswith(b"t"):
    got += os.read(0, 1)
os.write(1, b"\r\nreply=" + got.replace(b"\x1b", b"^") + b"\r\n")
"#;

fn screen(t: &PtyTerm) -> String {
    let mut cs = t.cells(false);
    cs.sort_by_key(|c| (c.row, c.col));
    cs.iter().map(|c| c.c).collect()
}

#[test]
fn a_program_reads_the_panes_pixel_size_from_the_pty() {
    let size = GridSize { cols: 80, rows: 24 };
    let mut t = PtyTerm::spawn_args(size, "python3", &["-c".into(), PROBE.into()]).unwrap();
    // As the app does on its first relayout, before the program has run.
    t.set_cell_px(9, 18);
    let mut w = t.writer();
    w.write_all(b"go\n").unwrap();
    w.flush().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !screen(&t).contains("reply=^[6;18;9t") {
        t.try_read();
        std::thread::sleep(Duration::from_millis(20));
    }
    let s = screen(&t);
    assert!(s.contains("ws=80x24,720x432"), "pty size: {}", s.trim());
    assert!(s.contains("reply=^[6;18;9t"), "16t reply: {}", s.trim());
}
