use super::jump_target;

/// A 1,000-line buffer on a 20-row window: 980 lines of history, the live
/// screen's top row is line 980. Commands started at lines 100, 400 and 700.
const STARTS: &[usize] = &[100, 400, 700];
const NOW: usize = 1000;
const ROWS: usize = 20;
const HIST: usize = 980;

fn jump(off: usize, up: bool) -> Option<usize> {
    jump_target(STARTS, NOW, ROWS, HIST, off, up)
}

#[test]
fn up_walks_back_one_command_start_per_press() {
    // Line 700 on the top row is offset 980 - 700 = 280.
    assert_eq!(jump(0, true), Some(280));
    assert_eq!(jump(280, true), Some(580));
    assert_eq!(jump(580, true), Some(880));
    // Nothing older: the key does nothing.
    assert_eq!(jump(880, true), None);
}

#[test]
fn up_from_between_two_starts_lands_on_the_one_above() {
    // Top row at line 500 (offset 480): the previous start is line 400.
    assert_eq!(jump(480, true), Some(580));
}

#[test]
fn down_walks_forward_then_back_to_the_live_screen() {
    assert_eq!(jump(880, false), Some(580));
    assert_eq!(jump(580, false), Some(280));
    assert_eq!(jump(280, false), Some(0));
    assert_eq!(jump(0, false), None);
}

#[test]
fn a_start_on_the_live_screen_is_reached_at_the_bottom() {
    // Line 990 is on the live screen: offset 0, not a wrapped-around number.
    let starts = [100, 990];
    assert_eq!(jump_target(&starts, NOW, ROWS, HIST, 0, true), Some(880));
    assert_eq!(jump_target(&starts, NOW, ROWS, HIST, 880, false), Some(0));
}

#[test]
fn a_start_older_than_the_history_is_skipped() {
    // The scrollback threw line 0 away (its offset, 980, is past the 500
    // kept): the oldest start still held, line 700, is as far up as it goes.
    let starts = [0, 700];
    assert_eq!(jump_target(&starts, NOW, ROWS, 500, 0, true), Some(280));
    assert_eq!(jump_target(&starts, NOW, ROWS, 500, 280, true), None);
}

#[test]
fn no_commands_means_up_does_nothing() {
    assert_eq!(jump_target(&[], NOW, ROWS, HIST, 0, true), None);
    assert_eq!(jump_target(&[], NOW, ROWS, HIST, 300, false), Some(0));
}

/// The chord itself, through `arrow_chord` on a real shell: Opt+↑ on the
/// screens' chord scrolls the terminal to a command start and leaves focus
/// where it was (before, Cmd+Opt+↑ walked the grid like Cmd+↑).
#[test]
fn the_screens_chord_up_scrolls_the_focused_terminal_to_a_command() {
    use crate::app::CrewApp;
    use crate::pane::{spawn_pane, PaneContent};
    use winit::keyboard::{Key, ModifiersState, NamedKey};
    let grid = crew_term::GridSize { cols: 40, rows: 10 };
    let dir = std::env::temp_dir();
    let mut app = CrewApp::default();
    for _ in 0..2 {
        app.panes
            .push(spawn_pane("sh", "sh", grid, Some(&dir)).unwrap());
    }
    let PaneContent::Terminal(t) = &mut app.panes[0].content else {
        panic!("not a terminal");
    };
    {
        use std::io::Write;
        let script = b"i=0; while [ $i -lt 60 ]; do echo line$i; i=$((i+1)); done\n";
        let _ = t.input.write_all(script);
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while t.pty.history_lines() < 40 && std::time::Instant::now() < deadline {
        t.pty.try_read();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(
        t.pty.history_lines() >= 40,
        "the shell never filled the history"
    );
    // A command started 30 lines above the live screen's top row.
    let from = t.pty.lines_seen() - 10 - 30;
    t.spans.started("build".into(), from, 0);
    app.focused = 0;
    let mods = match cfg!(target_os = "macos") {
        true => ModifiersState::SUPER | ModifiersState::ALT,
        false => ModifiersState::CONTROL | ModifiersState::ALT,
    };
    let up = Key::Named(NamedKey::ArrowUp);
    assert!(app.arrow_chord(&up, mods), "the chord is crew's");
    assert_eq!(app.focused, 0, "focus did not move");
    let PaneContent::Terminal(t) = &app.panes[0].content else {
        unreachable!()
    };
    assert_eq!(
        t.pty.display_offset(),
        30,
        "the command's start is the top row"
    );
}

/// Once the scrollback is full the jump still lands on the command's line:
/// a 20-line scrollback, a mark on the screen's top row, 12 more lines, and
/// Opt+↑ must show that same row on top again. A mark the scrollback has
/// since thrown away is no longer a place to jump to.
#[test]
fn a_mark_in_a_full_scrollback_still_lands_on_its_line() {
    use crate::app::CrewApp;
    use crate::pane::{spawn_pane, PaneContent, TermPane};
    let grid = crew_term::GridSize { cols: 40, rows: 10 };
    let mut app = CrewApp::default();
    let pane = spawn_pane("sh", "sh", grid, Some(&std::env::temp_dir())).unwrap();
    app.panes.push(pane);
    let PaneContent::Terminal(t) = &mut app.panes[0].content else {
        panic!("not a terminal");
    };
    t.pty.set_scrollback(20);
    use crew_term::TermModel;
    let text = |t: &TermPane| crate::search::rows_text(&t.pty.cells(false), 40, 10, false);
    let top = |t: &TermPane| text(t)[0].clone();
    // `n` lines of `p<i>`, then `done`, and wait until `done` is on screen.
    let run = |t: &mut TermPane, p: &str, n: usize, done: &str| {
        use std::io::Write;
        let s =
            format!("i=0; while [ $i -lt {n} ]; do echo {p}$i; i=$((i+1)); done; echo {done}\n");
        let _ = t.input.write_all(s.as_bytes());
        let end = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let seen = |t: &TermPane| text(t).iter().any(|r| r.starts_with(done));
        while !seen(t) && std::time::Instant::now() < end {
            t.pty.try_read();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(seen(t), "{done} never arrived");
    };
    run(t, "a", 60, "ONE");
    let marked = top(t);
    t.spans.started("build".into(), t.pty.lines_seen() - 10, 0);
    run(t, "b", 12, "TWO");
    assert!(app.jump_command(true));
    let PaneContent::Terminal(t) = &mut app.panes[0].content else {
        unreachable!()
    };
    assert!(t.pty.display_offset() > 0, "the jump went nowhere");
    assert_eq!(top(t), marked, "the mark's own line is on top");
    t.pty.scroll(-(t.pty.display_offset() as i32));
    run(t, "c", 40, "SIX");
    assert!(app.jump_command(true));
    let PaneContent::Terminal(t) = &app.panes[0].content else {
        unreachable!()
    };
    assert_eq!(t.pty.display_offset(), 0, "an evicted mark is not a target");
}
