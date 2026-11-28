//! A command's output cut out by its marks, fed as real OSC 133 bytes (see
//! `markpos` for the rule). Edge cases live in `modelcopyedge_tests`.
use crate::{CommandOutput, GridSize, HeadlessTerm, OutputErr, TermModel};

pub(super) const A: &str = "\x1b]133;A\x07";
pub(super) const B: &str = "\x1b]133;B\x07";
pub(super) const C: &str = "\x1b]133;C\x07";
pub(super) const D: &str = "\x1b]133;D;0\x07";

pub(super) fn term(cols: u16, rows: u16) -> HeadlessTerm {
    HeadlessTerm::new(GridSize { cols, rows })
}

/// What a zsh with integration writes for one command, as one read.
pub(super) fn run(cmd: &str, out: &str) -> String {
    format!("{cmd}\r\n{C}{out}{D}{A}$ {B}")
}

pub(super) fn lines(n: usize, tag: &str) -> String {
    (1..=n).map(|i| format!("{tag}{i}\r\n")).collect()
}

pub(super) fn ok(t: &HeadlessTerm) -> CommandOutput {
    t.command_output().expect("a command to copy")
}

#[test]
fn a_fast_command_after_a_slow_one_is_the_one_copied() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(run("cargo test", &lines(30, "cargo ")).as_bytes());
    t.feed(run("ls", "a.txt\r\nb.txt\r\n").as_bytes());
    let got = ok(&t);
    assert_eq!(
        (got.command.as_str(), got.text.as_str()),
        ("ls", "a.txt\nb.txt")
    );
    assert_eq!(got.dropped, 0);
}

#[test]
fn the_head_of_output_that_arrived_in_one_read_is_kept() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(run("make", &lines(200, "row ")).as_bytes());
    let got = ok(&t);
    assert!(got.text.starts_with("row 1\nrow 2\n"), "{}", got.text);
    assert!(got.text.ends_with("row 200"));
    assert_eq!(got.text.lines().count(), 200);
}

#[test]
fn a_screen_that_never_scrolled_still_copies() {
    let mut t = term(40, 20);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(run("ls", "one\r\ntwo\r\n").as_bytes());
    assert_eq!(t.history_lines(), 0);
    assert_eq!(ok(&t).text, "one\ntwo");
}

#[test]
fn clear_prints_nothing_rather_than_the_command_before_it() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(run("ls", "one\r\n").as_bytes());
    t.feed(run("clear", "\x1b[H\x1b[2J\x1b[3J").as_bytes());
    assert_eq!(t.command_output(), Err(OutputErr::Nothing("clear".into())));
    t.feed(run("pwd", "/tmp\r\n").as_bytes());
    assert_eq!(ok(&t).text, "/tmp");
}

#[test]
fn a_command_with_no_marks_has_nothing_to_copy() {
    let mut t = term(40, 10);
    t.feed(b"$ ls\r\none\r\n");
    assert_eq!(t.command_output(), Err(OutputErr::NoMarks));
}

#[test]
fn a_prompt_without_done_closes_the_command_too() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}ls\r\n{C}one\r\n{A}$ {B}").as_bytes());
    assert_eq!(ok(&t).text, "one");
}

#[test]
fn a_partial_last_line_and_a_mid_line_c_are_read_as_documented() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}echo -n hi{C}\r\nhi{D}{A}$ {B}").as_bytes());
    assert_eq!(ok(&t).text, "hi");
}

#[test]
fn the_text_is_plain_and_wide_characters_whole() {
    let mut t = term(40, 10);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(run("ls", "\x1b[31m日本\x1b[0m ok\r\n").as_bytes());
    assert_eq!(ok(&t).text, "日本 ok");
}

#[test]
fn a_wrap_right_after_a_space_keeps_the_space() {
    let mut t = term(10, 10);
    t.feed(format!("{A}$ {B}").as_bytes());
    t.feed(run("echo", "aaaa bbbb cccc\r\n").as_bytes());
    assert_eq!(ok(&t).text, "aaaa bbbb cccc");
}

#[test]
fn the_command_is_what_was_typed_after_the_prompt() {
    let mut t = term(40, 10);
    t.feed(format!("{A}~/code $ {B}").as_bytes());
    t.feed(run("git status", "clean\r\n").as_bytes());
    assert_eq!(ok(&t).command, "git status");
}

#[test]
fn output_that_exactly_fills_a_row_is_whole() {
    let mut t = term(10, 10);
    t.feed(format!("{A}$ {B}echo -n x\r\n{C}abcdefghij{D}{A}$ {B}").as_bytes());
    assert_eq!(ok(&t).text, "abcdefghij");
}
