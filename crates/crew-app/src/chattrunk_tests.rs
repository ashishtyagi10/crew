//! A task's chained replies hang off one trunk.
use super::*;

fn reply(text: &str) -> Message {
    Message {
        sender: "coder".into(),
        text: text.into(),
        ts: String::new(),
        meta: "task:7".into(),
        usage: None,
        expanded: false,
    }
}

fn texts(msgs: &[Message], cols: usize) -> Vec<String> {
    let refs: Vec<&Message> = msgs.iter().collect();
    let lines = card_lines(&refs, cols, 0, View::default());
    lines
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect())
        .collect()
}

#[test]
fn the_tees_join_through_the_bodies_between_them() {
    let t = texts(&[reply("root"), reply("mid"), reply("last")], 80);
    let row = |s: &str| {
        t.iter()
            .find(|l| l.trim_start().ends_with(s))
            .unwrap()
            .clone()
    };
    // The root's body is its own; a middle reply's body sits on the trunk;
    // the last reply's body hangs clear of it, under the `└`.
    assert_eq!(row("root"), " root", "{t:?}");
    assert_eq!(row("mid"), "\u{2502}mid", "{t:?}");
    assert_eq!(row("last"), " last", "{t:?}");
}

#[test]
fn a_chained_body_wraps_inside_the_hang() {
    let long = "word ".repeat(30);
    let t = texts(&[reply("root"), reply(&long), reply("last")], 40);
    for l in t.iter().filter(|l| l.contains("word")) {
        assert!(l.starts_with('\u{2502}'), "{l:?}");
        assert!(crate::chatwidth::str_w(l) <= 40, "{l:?}");
    }
}

#[test]
fn a_code_block_in_a_chained_reply_still_copies_clean() {
    let _g = crate::app::theme_test_guard();
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    let mut pane = crate::chat::ChatPane::new(plugin, "crew".into());
    let code = "Here:\n\n```rust\nfn a() {}\nfn b() {}\n```\n\nDone.";
    pane.messages = vec![reply("root"), reply(code), reply("last")];
    let (cols, rows) = (60u16, 40u16);
    let placed = crate::chatplace::placed_lines(&pane, cols, rows);
    let row = placed
        .iter()
        .find(|(_, l)| l.iter().map(|c| c.c).collect::<String>().contains("fn a()"))
        .map(|(r, _)| *r)
        .expect("the code line is on screen");
    let got = crate::chatview::code_block_at(&pane, cols, rows, row).expect("a block");
    assert_eq!(got, "fn a() {}\nfn b() {}", "{got:?}");
}
