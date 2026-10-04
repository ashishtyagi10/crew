use winit::keyboard::Key;

use crate::chat::tests::pane;
use crate::chatkeys::{chat_key, ChatAction, ChatInput};

fn ctrl(c: &str) -> ChatInput {
    chat_key(&Key::Character(c.into()), true, false, true)
}

/// Ctrl+letters are chords: C, D and J mean something, the rest nothing —
/// none of them types its letter. What else Ctrl makes (AltGr off the Mac)
/// still types.
#[test]
fn ctrl_letters_are_chords_and_altgr_still_types() {
    assert_eq!(ctrl("c"), ChatInput::Cancel);
    assert_eq!(ctrl("D"), ChatInput::EndOfInput);
    assert_eq!(ctrl("j"), ChatInput::Newline);
    assert_eq!(ctrl("l"), ChatInput::Ignore);
    assert_eq!(ctrl("@"), ChatInput::Char('@'));
    assert_eq!(ctrl("€"), ChatInput::Char('€'));
}

/// Ctrl+C clears what is typed; on an empty composer it stops a running
/// turn, as Esc does (the stop clock is armed: `chatwatch`).
#[test]
fn ctrl_c_clears_then_interrupts() {
    let cwd = std::env::temp_dir();
    let mut p = pane();
    p.input = "half a thought".into();
    p.on_input(ChatInput::Cancel, &cwd);
    assert!(p.input.is_empty());
    assert!(p.watch.stop_asked.is_none(), "clearing is not stopping");
    p.connected = true;
    p.awaiting = true;
    p.on_input(ChatInput::Cancel, &cwd);
    assert!(
        p.watch.stop_asked.is_some(),
        "an empty composer's Ctrl+C stops the turn"
    );
}

/// Ctrl+D closes an idle pane from an empty composer, and nothing else.
#[test]
fn ctrl_d_closes_only_an_empty_idle_pane() {
    let cwd = std::env::temp_dir();
    let mut p = pane();
    assert!(matches!(
        p.on_input(ChatInput::EndOfInput, &cwd),
        Some(ChatAction::Close)
    ));
    p.input = "x".into();
    assert!(p.on_input(ChatInput::EndOfInput, &cwd).is_none());
    p.input.clear();
    p.awaiting = true;
    assert!(
        p.on_input(ChatInput::EndOfInput, &cwd).is_none(),
        "not while busy"
    );
}

/// A `\` before Enter, and Ctrl+J, start a new line instead of sending.
#[test]
fn backslash_enter_and_ctrl_j_insert_a_newline() {
    let cwd = std::env::temp_dir();
    let mut p = pane();
    p.input = "first line\\".into();
    p.on_input(ChatInput::Enter, &cwd);
    assert_eq!(p.input, "first line\n");
    p.input.push_str("second");
    p.on_input(ctrl("j"), &cwd);
    assert_eq!(p.input, "first line\nsecond\n");
}

/// `?` on an empty composer opens the key reference; anywhere else it types.
#[test]
fn a_question_mark_on_an_empty_composer_opens_the_keys() {
    let cwd = std::env::temp_dir();
    let mut p = pane();
    assert!(matches!(
        p.on_input(ChatInput::Char('?'), &cwd),
        Some(ChatAction::Help)
    ));
    p.input = "why".into();
    p.on_input(ChatInput::Char('?'), &cwd);
    assert_eq!(p.input, "why?");
}
