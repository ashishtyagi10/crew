use std::ffi::OsStr;

use portable_pty::CommandBuilder;

use super::identify;

#[test]
fn a_pane_names_crew_with_24_bit_colour_and_no_other_terminal() {
    let mut cmd = CommandBuilder::new("sh");
    // What crew inherits when another terminal started it.
    cmd.env("TERM_PROGRAM", "iTerm.app");
    cmd.env("LC_TERMINAL", "iTerm2");
    cmd.env("KITTY_WINDOW_ID", "3");
    cmd.env("TMUX", "/tmp/tmux-501/default,1,0");
    cmd.env("HOME", "/Users/x");
    identify(&mut cmd);
    let get = |k: &str| cmd.get_env(k).map(OsStr::to_string_lossy);
    assert_eq!(get("COLORTERM").as_deref(), Some("truecolor"));
    assert_eq!(get("TERM_PROGRAM").as_deref(), Some("crew"));
    assert_eq!(
        get("TERM_PROGRAM_VERSION").as_deref(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    for gone in ["LC_TERMINAL", "KITTY_WINDOW_ID", "TMUX"] {
        assert_eq!(get(gone), None, "{gone} is another terminal's");
    }
    assert_eq!(get("HOME").as_deref(), Some("/Users/x"), "the rest stays");
}

/// Through a real shell in a real pty: what `$COLORTERM` and `$TERM_PROGRAM`
/// read inside a pane.
#[cfg(unix)]
#[test]
fn a_shell_in_a_pane_reads_crew_and_truecolor() {
    use std::io::Write;
    use std::time::{Duration, Instant};

    use crate::{GridSize, PtyTerm, TermModel};

    let mut term = PtyTerm::spawn(GridSize { cols: 60, rows: 6 }, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"printf 'id:%s:%s:\\n' \"$COLORTERM\" \"$TERM_PROGRAM\"\n")
        .unwrap();
    w.flush().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = String::new();
    while Instant::now() < deadline && !seen.contains("id:truecolor:crew:") {
        term.try_read();
        let mut cs = term.cells(true);
        cs.sort_by_key(|c| (c.row, c.col));
        seen = cs.iter().map(|c| c.c).collect();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(seen.contains("id:truecolor:crew:"), "{seen:?}");
}
