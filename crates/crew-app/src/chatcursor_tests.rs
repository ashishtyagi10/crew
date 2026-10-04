use winit::keyboard::{Key, NamedKey};

use super::{apply, reduce_at, Op};
use crate::chat::tests::pane;
use crate::chatkeys::{chat_key_alt, ChatInput};
use crew_theme::deco::CursorShape;

/// Apply `op` to `text` with the caret at `|`; the result with its caret.
fn op(text: &str, op: Op) -> String {
    let at = text.find('|').unwrap();
    let mut chars: Vec<char> = text.replace('|', "").chars().collect();
    let to = apply(&mut chars, text[..at].chars().count(), op);
    chars.insert(to, '|');
    chars.into_iter().collect()
}

#[test]
fn every_op_moves_or_deletes_around_the_caret() {
    assert_eq!(op("ab|c", Op::Left), "a|bc");
    assert_eq!(op("|abc", Op::Left), "|abc");
    assert_eq!(op("ab|c", Op::Right), "abc|");
    assert_eq!(op("fix the te|sts", Op::WordLeft), "fix the |tests");
    assert_eq!(op("fix |the tests", Op::WordRight), "fix the| tests");
    assert_eq!(op("one\ntw|o\nthree", Op::LineStart), "one\n|two\nthree");
    assert_eq!(op("one\ntw|o\nthree", Op::LineEnd), "one\ntwo|\nthree");
    assert_eq!(op("ab|cd", Op::Delete), "ab|d");
    assert_eq!(op("fix the te|sts", Op::WordBack), "fix the |sts");
    assert_eq!(op("one\ntw|o\nthree", Op::KillEnd), "one\ntw|\nthree");
    assert_eq!(
        op("one|\ntwo", Op::KillEnd),
        "one|two",
        "at a line's end, the break"
    );
    assert_eq!(op("one\ntw|o", Op::KillStart), "one\n|o");
}

#[test]
fn typing_and_backspace_land_at_the_caret() {
    let mut s = String::from("held");
    assert_eq!(reduce_at(&mut s, 2, Some('l'), false, false), None);
    assert_eq!(s, "hel".to_string() + "ld");
    reduce_at(&mut s, 2, None, false, true);
    assert_eq!(s, "held");
    assert_eq!(
        reduce_at(&mut s, 2, None, true, false).as_deref(),
        Some("held")
    );
    assert!(s.is_empty());
}

/// The keys: ← and the line ends plain, a word with Alt, the emacs chords.
#[test]
fn the_caret_keys_are_decoded() {
    let k = |key: Key, alt: bool, ctrl: bool| chat_key_alt(&key, true, (false, ctrl, alt));
    let named = |n| Key::Named(n);
    assert_eq!(
        k(named(NamedKey::ArrowLeft), false, false),
        ChatInput::Caret(Op::Left)
    );
    assert_eq!(
        k(named(NamedKey::ArrowLeft), true, false),
        ChatInput::Caret(Op::WordLeft)
    );
    assert_eq!(
        k(named(NamedKey::ArrowRight), true, false),
        ChatInput::Caret(Op::WordRight)
    );
    assert_eq!(
        k(named(NamedKey::ArrowRight), false, false),
        ChatInput::Accept
    );
    assert_eq!(
        k(named(NamedKey::Backspace), true, false),
        ChatInput::Caret(Op::WordBack)
    );
    assert_eq!(
        k(named(NamedKey::Home), false, false),
        ChatInput::Caret(Op::LineStart)
    );
    assert_eq!(
        k(named(NamedKey::End), false, false),
        ChatInput::Caret(Op::LineEnd)
    );
    assert_eq!(
        k(named(NamedKey::Delete), false, false),
        ChatInput::Caret(Op::Delete)
    );
    let c = |s: &str| k(Key::Character(s.into()), false, true);
    assert_eq!(c("a"), ChatInput::Caret(Op::LineStart));
    assert_eq!(c("e"), ChatInput::Caret(Op::LineEnd));
    assert_eq!(c("k"), ChatInput::Caret(Op::KillEnd));
    assert_eq!(c("u"), ChatInput::Caret(Op::KillStart));
    assert_eq!(c("w"), ChatInput::Caret(Op::WordBack));
}

/// Fix a word in the middle of a draft: move back, type, delete — the text
/// after the caret stays put, and so does the caret relative to it.
#[test]
fn a_draft_is_edited_in_the_middle() {
    let cwd = std::env::temp_dir();
    let mut p = pane();
    for c in "fix the tests".chars() {
        p.on_typed(ChatInput::Char(c), &cwd);
    }
    p.on_typed(ChatInput::Caret(Op::WordLeft), &cwd);
    for c in "failing ".chars() {
        p.on_typed(ChatInput::Char(c), &cwd);
    }
    assert_eq!(p.input, "fix the failing tests");
    assert_eq!(p.caret_back, "tests".len());
    p.on_typed(ChatInput::Backspace, &cwd);
    assert_eq!(p.input, "fix the failingtests");
    p.on_typed(ChatInput::Caret(Op::LineStart), &cwd);
    p.on_typed(ChatInput::Caret(Op::Delete), &cwd);
    assert_eq!(p.input, "ix the failingtests");
    p.paste_composer("f");
    assert_eq!(
        p.input, "fix the failingtests",
        "a paste lands at the caret"
    );
}

/// → moves the caret until it reaches the end; there, it takes the
/// suggestion as it always did — which mid-draft is not shown at all.
#[test]
fn right_moves_until_the_end_and_the_suggestion_waits_for_it() {
    let cwd = std::env::temp_dir();
    let mut p = pane();
    p.history.record("run the tests");
    for c in "run".chars() {
        p.on_typed(ChatInput::Char(c), &cwd);
    }
    p.on_typed(ChatInput::Caret(Op::Left), &cwd);
    assert!(p.ghost().is_none(), "no suggestion mid-draft");
    p.on_typed(ChatInput::Accept, &cwd);
    assert_eq!((p.input.as_str(), p.caret_back), ("run", 0), "→ only moved");
    p.on_typed(ChatInput::Accept, &cwd);
    assert_eq!(p.input, "run the tests", "at the end → takes it");
}

/// A key that replaces the draft (a recalled line) puts the caret at the end.
#[test]
fn a_recalled_line_puts_the_caret_at_the_end() {
    let cwd = std::env::temp_dir();
    let mut p = pane();
    p.history.record("an older prompt");
    p.input = "an old".into(); // Up recalls what starts with the draft
    p.caret_back = 3;
    p.on_typed(ChatInput::Up, &cwd);
    assert_eq!((p.input.as_str(), p.caret_back), ("an older prompt", 0));
}

/// Drawn: mid-draft, the glyph after the caret carries a beam and no `▏`
/// cell is drawn; at the end, the `▏` follows the text as before. In a draft
/// taller than the composer, the caret's line is the one kept on screen.
#[test]
fn the_caret_is_drawn_where_it_is() {
    let draw = |input: &str, caret: usize| {
        crate::chatinput::composer_cells_at(input, caret, None, &[], 40, 30)
    };
    let mid = draw("hello world", 6);
    let beamed: Vec<char> = mid
        .iter()
        .filter(|c| c.cursor.shape == CursorShape::Beam)
        .map(|c| c.c)
        .collect();
    assert_eq!(beamed, ['w']);
    assert!(!mid.iter().any(|c| c.c == '\u{258f}'));
    let end = draw("hello world", 11);
    assert!(end.iter().any(|c| c.c == '\u{258f}'));
    let tall: String = (0..30).map(|i| format!("line{i}\n")).collect();
    let first = draw(&tall, 2); // in "line0"
    assert!(first
        .iter()
        .any(|c| c.cursor.shape == CursorShape::Beam && c.c == 'n'));
}
