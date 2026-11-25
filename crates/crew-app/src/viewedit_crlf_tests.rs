//! Editing a text file whose bytes are rewritten for the screen — CRLF line ends, tabs — lands
//! every keystroke on the file's own bytes.
use winit::keyboard::NamedKey;

use super::code_tests::{app_with, ask, named, save, typed};
use crate::viewpane::keys::ViewInput;

/// CRLF: a blank line's text goes before its `\r`, and Enter writes `\r\n` — the file stays CRLF.
#[test]
fn crlf_blank_lines_and_enter_stay_crlf() {
    let (mut app, f) = app_with("crlf.txt", "a\r\n\r\nb\r\n");
    ask(&mut app, ViewInput::Insert);
    named(&mut app, NamedKey::ArrowDown);
    typed(&mut app, "x");
    save(&mut app);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "a\r\nx\r\nb\r\n");
    let (mut app, f) = app_with("crlf2.txt", "a\r\n");
    ask(&mut app, ViewInput::Insert);
    named(&mut app, NamedKey::End);
    named(&mut app, NamedKey::Enter);
    typed(&mut app, "y");
    save(&mut app);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "a\r\ny\r\n");
}

/// A tab-indented line can be typed at column 0, before its tab — and a line of only a tab too.
#[test]
fn a_tab_indented_line_starts_before_its_tab() {
    let (mut app, f) = app_with("tabs.mk", "x\n\tb\n\t\n");
    ask(&mut app, ViewInput::Insert);
    named(&mut app, NamedKey::ArrowDown);
    named(&mut app, NamedKey::Home);
    typed(&mut app, "#");
    named(&mut app, NamedKey::ArrowDown);
    named(&mut app, NamedKey::Home);
    typed(&mut app, "@");
    save(&mut app);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "x\n#\tb\n@\t\n");
}
