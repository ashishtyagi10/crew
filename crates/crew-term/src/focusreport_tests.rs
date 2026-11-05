use super::*;
use crate::{GridSize, HeadlessTerm, TermModel};
use std::io::Write;
use std::time::{Duration, Instant};

const IN: Option<&[u8]> = Some(b"\x1b[I");
const OUT: Option<&[u8]> = Some(b"\x1b[O");

#[test]
fn reports_only_when_the_answer_changes() {
    // (told, now, enabled) -> (told', bytes)
    let rows: &[(Option<bool>, bool, bool, Option<bool>, Option<&[u8]>)] = &[
        (None, true, false, None, None),
        (Some(true), false, false, None, None),
        (None, true, true, Some(true), None),
        (None, false, true, Some(false), OUT),
        (Some(true), true, true, Some(true), None),
        (Some(true), false, true, Some(false), OUT),
        (Some(false), false, true, Some(false), None),
        (Some(false), true, true, Some(true), IN),
    ];
    for &(told, now, on, want_told, want) in rows {
        assert_eq!(
            focus_report(told, now, on),
            (want_told, want),
            "told={told:?} now={now} on={on}"
        );
    }
}

#[test]
fn decset_1004_shows_in_input_modes() {
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
    assert!(!t.input_modes().focus_events);
    t.feed(b"\x1b[?1004h");
    assert!(t.input_modes().focus_events);
    t.feed(b"\x1b[?1004l");
    assert!(!t.input_modes().focus_events);
}

fn pump_until(term: &mut PtyTerm, done: impl Fn(&PtyTerm) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        term.try_read();
        if done(term) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

fn screen(term: &PtyTerm) -> String {
    let mut cs = term.cells(true);
    cs.sort_by_key(|c| (c.row, c.col));
    cs.iter().map(|c| c.c).collect()
}

/// A real program on a real pty: it turns 1004 on, crew drops its focus, and
/// the program reads `ESC [ O` from its stdin — once, not every tick.
#[test]
fn a_program_that_asked_reads_focus_out() {
    let mut term = PtyTerm::spawn(GridSize { cols: 60, rows: 8 }, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"printf '\\033[?1004h'; IFS= read -r x; printf '%s' \"$x\" | od -An -tx1\n")
        .unwrap();
    w.flush().unwrap();
    assert!(pump_until(&mut term, |t| t.input_modes().focus_events));
    assert!(!term.report_focus(true), "focused at enable: latch only");
    assert!(term.report_focus(false));
    assert!(!term.report_focus(false), "unchanged: nothing written");
    w.write_all(b"\n").unwrap();
    w.flush().unwrap();
    assert!(
        pump_until(&mut term, |t| screen(t).contains("1b5b4f")),
        "program never read CSI O: {}",
        screen(&term)
    );
}
