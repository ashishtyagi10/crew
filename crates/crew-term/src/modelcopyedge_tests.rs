//! Where the marks' lines stop being trustworthy, and where they must keep
//! being right: reads cut anywhere, screens that move, history that goes.
use super::modelcopy_tests::{lines, ok, run, term, A, B, C, D};
use crate::{OutputErr, TermModel};

#[test]
fn marks_split_across_reads_land_on_the_same_lines_at_every_offset() {
    for st in [false, true] {
        let enc = |s: &str| match st {
            true => s.replace('\x07', "\x1b\\"),
            false => s.to_string(),
        };
        let stream = enc(&format!(
            "{A}$ {B}{}{}",
            run("one", &lines(25, "x ")),
            run("two", &lines(12, "y "))
        ));
        let mut whole = term(20, 10);
        whole.feed(stream.as_bytes());
        let want = (whole.core.marks.dump(), ok(&whole));
        for at in 0..stream.len() {
            let mut t = term(20, 10);
            t.feed(&stream.as_bytes()[..at]);
            t.feed(&stream.as_bytes()[at..]);
            assert_eq!((t.core.marks.dump(), ok(&t)), want, "split at {at} st={st}");
        }
    }
}

#[test]
fn a_full_screen_program_refuses_and_leaves_no_garbage_after() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(format!("vim a\r\n{C}\x1b[?1049h\x1b[Hjunk").as_bytes());
    assert_eq!(t.command_output(), Err(OutputErr::AltScreen));
    t.feed(format!("\x1b[?1049l{D}{A}$ {B}").as_bytes());
    assert_eq!(t.command_output(), Err(OutputErr::Nothing("vim a".into())));
    // A dead program's alternate screen, closed by `D` there.
    t.feed(format!("vim b\r\n{C}\x1b[?1049h\x1b[Hjunk{D}{A}$ {B}").as_bytes());
    assert_eq!(t.command_output(), Err(OutputErr::AltScreen));
}

#[test]
fn a_width_change_refuses_but_a_height_change_does_not() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(run("ls", &lines(30, "r ")).as_bytes());
    let before = ok(&t).text;
    t.resize(crate::GridSize { cols: 40, rows: 6 });
    assert_eq!(ok(&t).text, before);
    t.resize(crate::GridSize { cols: 40, rows: 14 });
    assert_eq!(ok(&t).text, before);
    t.resize(crate::GridSize { cols: 30, rows: 14 });
    assert_eq!(t.command_output(), Err(OutputErr::Resized));
    t.resize(crate::GridSize { cols: 40, rows: 14 });
    assert_eq!(t.command_output(), Err(OutputErr::Resized));
    t.feed(run("pwd", "/tmp\r\n").as_bytes());
    assert_eq!(ok(&t).text, "/tmp");
}

#[test]
fn a_shrinking_screen_over_a_full_scrollback_keeps_the_lines_straight() {
    let mut t = term(40, 10);
    t.set_scrollback(20);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(run("ls", &lines(60, "r ")).as_bytes());
    for rows in [4, 10, 3] {
        t.resize(crate::GridSize { cols: 40, rows });
        let got = ok(&t);
        assert!(got.text.ends_with("r 60"), "{rows}: {}", got.text);
        assert_eq!(
            got.text.lines().count() as u64 + got.dropped,
            60,
            "{rows}: every line is either kept or counted dropped"
        );
    }
}

#[test]
fn lines_the_scrollback_threw_away_are_counted_or_refused() {
    let mut t = term(40, 10);
    t.set_scrollback(30);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(run("make", &lines(100, "r ")).as_bytes());
    let got = ok(&t);
    assert!(got.dropped > 50, "{}", got.dropped);
    assert_eq!(got.text.lines().count() as u64 + got.dropped, 100);
    assert!(got.text.ends_with("r 100"));
    t.feed(lines(200, "noise ").as_bytes());
    assert_eq!(t.command_output(), Err(OutputErr::Gone("make".into())));
}

#[test]
fn clearing_the_history_keeps_later_lines_numbered() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(lines(40, "old ").as_bytes());
    let before = t.screen_line();
    t.feed(b"\x1b[3J");
    assert_eq!(t.screen_line(), before);
    t.feed(run("ls", "fresh\r\n").as_bytes());
    assert_eq!(ok(&t).text, "fresh");
}

#[test]
fn a_synchronized_update_holding_the_cursor_back_is_not_trusted() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}ls\r\n{C}one\r\n\x1b[?2026hx{D}").as_bytes());
    assert_eq!(t.command_output(), Err(OutputErr::Unsure));
}

#[test]
fn scrolled_back_the_command_on_view_is_the_one_copied() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}").as_bytes());
    for (cmd, tag) in [("one", "a "), ("two", "b "), ("three", "c ")] {
        t.feed(run(cmd, &lines(30, tag)).as_bytes());
    }
    assert_eq!(ok(&t).command, "three");
    // The wheel: a few lines up, still inside `three`'s output.
    t.scroll(5);
    assert_eq!(ok(&t).command, "three");
    // Far enough that `two`'s output fills the view and `three` is below.
    t.scroll(30);
    assert_eq!(ok(&t).command, "two");
    t.scroll(30);
    let got = ok(&t);
    assert_eq!(
        (got.command.as_str(), got.text.lines().count()),
        ("one", 30)
    );
    t.scroll(-1000);
    assert_eq!(ok(&t).command, "three");
}
