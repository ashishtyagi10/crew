//! OSC 52 reads through the whole terminal, as a program on the pty asks.
use crate::model::{GridSize, HeadlessTerm, TermModel};

fn term() -> HeadlessTerm {
    HeadlessTerm::new(GridSize { cols: 80, rows: 24 })
}

/// A read query becomes an ask the app can answer, and the answer is the
/// clipboard in base64, closed with the BEL the query used.
#[test]
fn a_bel_query_asks_and_answers_with_bel() {
    let mut t = term();
    t.feed(b"\x1b]52;c;?\x07");
    let ask = t.take_clipboard_ask().expect("the read reaches the app");
    assert_eq!(ask.reply("hi"), "\x1b]52;c;aGk=\x07");
    assert!(t.take_clipboard_ask().is_none(), "taken once");
    assert_eq!(t.take_replies(), None, "the term itself answers nothing");
}

/// tmux and nvim close OSC with ST; the answer closes the same way.
#[test]
fn an_st_query_answers_with_st() {
    let mut t = term();
    t.feed(b"\x1b]52;c;?\x1b\\");
    let ask = t.take_clipboard_ask().expect("asked");
    assert_eq!(ask.reply("hi"), "\x1b]52;c;aGk=\x1b\\");
}

/// A flood of queries leaves one ask, not one per query.
#[test]
fn many_queries_leave_one_ask() {
    let mut t = term();
    t.feed(&b"\x1b]52;c;?\x07".repeat(1000));
    assert!(t.take_clipboard_ask().is_some());
    assert!(t.take_clipboard_ask().is_none());
}

/// The primary selection is not the clipboard: no ask for `p` or `s`.
#[test]
fn selection_reads_are_not_asked() {
    let mut t = term();
    t.feed(b"\x1b]52;p;?\x07\x1b]52;s;?\x07");
    assert!(t.take_clipboard_ask().is_none());
}

/// Copying still works alongside: a store is a store, not an ask.
#[test]
fn a_store_is_not_an_ask() {
    let mut t = term();
    t.feed(b"\x1b]52;c;aGk=\x07");
    assert!(t.take_clipboard_ask().is_none());
    assert_eq!(t.take_clipboard().as_deref(), Some("hi"));
}
