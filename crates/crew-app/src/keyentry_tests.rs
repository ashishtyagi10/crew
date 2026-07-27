use super::*;
use crate::chatkeys::ChatInput;

fn typed(e: &mut KeyEntry, s: &str) {
    for c in s.chars() {
        assert!(matches!(e.key(&ChatInput::Char(c)), KeyOutcome::Consumed));
    }
}

#[test]
fn typing_then_enter_submits_exactly_what_was_typed() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    typed(&mut e, "sk-test-not-a-real-key");
    match e.key(&ChatInput::Enter) {
        KeyOutcome::Submit(v) => assert_eq!(v, "sk-test-not-a-real-key"),
        _ => panic!("expected a submit"),
    }
}

#[test]
fn surrounding_whitespace_is_trimmed() {
    // Pasting a key commonly drags a trailing newline or space with it.
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    typed(&mut e, "  sk-padded  ");
    match e.key(&ChatInput::Enter) {
        KeyOutcome::Submit(v) => assert_eq!(v, "sk-padded"),
        _ => panic!("expected a submit"),
    }
}

#[test]
fn backspace_deletes_and_escape_cancels() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    typed(&mut e, "abc");
    assert!(matches!(e.key(&ChatInput::Backspace), KeyOutcome::Consumed));
    match e.key(&ChatInput::Enter) {
        KeyOutcome::Submit(v) => assert_eq!(v, "ab"),
        _ => panic!("expected a submit"),
    }
    let mut e2 = KeyEntry::new("ANTHROPIC_API_KEY".into());
    typed(&mut e2, "abc");
    assert!(matches!(e2.key(&ChatInput::Close), KeyOutcome::Cancelled));
}

#[test]
fn enter_on_an_empty_buffer_does_not_submit() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    assert!(matches!(e.key(&ChatInput::Enter), KeyOutcome::Consumed));
    typed(&mut e, "   ");
    assert!(matches!(e.key(&ChatInput::Enter), KeyOutcome::Consumed));
}

#[test]
fn the_popup_is_modal_and_swallows_other_keys() {
    // Arrows and Tab must not leak to the pane underneath while a secret is
    // half-typed.
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    for k in [
        ChatInput::Up,
        ChatInput::Down,
        ChatInput::Complete,
        ChatInput::Newline,
    ] {
        assert!(matches!(e.key(&k), KeyOutcome::Consumed), "{k:?} leaked");
    }
}

#[test]
fn the_card_masks_every_character_and_never_draws_the_secret() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    let secret = "sk-supersecret";
    typed(&mut e, secret);
    let cells = e.card(60);
    let drawn: String = cells.iter().map(|c| c.c).collect();
    for ch in secret.chars().filter(|c| !c.is_whitespace() && *c != '-') {
        assert!(
            !drawn.contains(ch) || "ANTHROPICKEY_".contains(ch),
            "character {ch:?} of the secret reached the screen"
        );
    }
    assert_eq!(
        cells.iter().filter(|c| c.c == '•').count(),
        secret.chars().count(),
        "one mask glyph per typed character"
    );
    assert!(
        drawn.contains("ANTHROPIC_API_KEY"),
        "the legend names the variable"
    );
}

#[test]
fn a_long_key_never_overflows_the_card() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    typed(&mut e, &"x".repeat(500));
    let cols = 40u16;
    let cells = e.card(cols);
    assert!(
        cells.iter().all(|c| c.col < cols),
        "a cell escaped the card"
    );
}
