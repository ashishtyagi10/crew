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
    // A mixed-case secret: a legend-agnostic assertion must catch a leak here
    // too, not just for an all-lowercase sample that happened to dodge an
    // all-caps legend.
    let secret = "sk-SUPERSECRET";
    typed(&mut e, secret);
    let cells = e.card(60);

    // The legend (row 0) is drawn text, not a leak risk by construction — the
    // buffer is only ever drawn on the first interior row (`row: 1`, see
    // `card`; with no sign-in in flight the card is border, input, border).
    // Scoping the assertion to that
    // row is legend-agnostic: it would catch a leak regardless of what the
    // legend says, unlike a global "does this character appear anywhere"
    // check, which false-positives whenever the secret shares a letter with
    // the legend text.
    let interior: String = cells.iter().filter(|c| c.row == 1).map(|c| c.c).collect();
    for ch in secret.chars() {
        assert!(
            !interior.contains(ch),
            "character {ch:?} of the secret reached the screen"
        );
    }
    assert_eq!(
        cells.iter().filter(|c| c.c == '•').count(),
        secret.chars().count(),
        "one mask glyph per typed character"
    );
    let legend: String = cells.iter().filter(|c| c.row == 0).map(|c| c.c).collect();
    assert!(
        legend.contains("ANTHROPIC_API_KEY"),
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

#[test]
fn a_waiting_prompt_says_so_and_still_masks_what_is_typed() {
    let mut e = KeyEntry::new("OPENROUTER_API_KEY".into());
    e.set_waiting(true);
    let drawn: String = e.card(60).iter().map(|c| c.c).collect();
    assert!(drawn.contains("waiting for browser"), "{drawn}");
    // Pasting must still work while the browser flow is in flight — it may
    // never have opened. `paste` (not `key`) deliberately, because typing
    // clears `waiting`, and the point of this test is the two coexisting.
    let secret = "sk-typed";
    e.paste(secret);
    let cells = e.card(60);

    // The hint must live on row 2 and NOWHERE else. That is what makes the
    // row-1 leak assertion below mean anything: the hint contains almost
    // every character of `secret`, so a hint drawn on row 1 would satisfy a
    // "row 1 doesn't contain the secret" check by accident while hiding a
    // real leak. Assert the invariant directly, both ways round.
    let hint = "waiting for browser";
    let row2: String = cells.iter().filter(|c| c.row == 2).map(|c| c.c).collect();
    assert!(
        row2.contains(hint),
        "the waiting hint belongs on row 2, alone: {row2:?}"
    );
    // Inside the side borders only — `titled_card` draws a `│` at each end of
    // every interior row and nothing else, so what remains is ours.
    let row1: Vec<char> = cells
        .iter()
        .filter(|c| c.row == 1 && c.col > 0 && c.col + 1 < 60)
        .map(|c| c.c)
        .collect();
    assert!(
        row1.iter().all(|c| *c == '•'),
        "row 1 holds mask glyphs and nothing else — no hint, no secret: {row1:?}"
    );
    assert_eq!(
        row1.len(),
        secret.chars().count(),
        "one mask glyph per pasted character"
    );
    let interior: String = row1.iter().collect();
    for ch in secret.chars() {
        assert!(
            !interior.contains(ch),
            "character {ch:?} reached the screen"
        );
    }
}

#[test]
fn the_card_is_only_as_tall_as_it_needs_the_hint_row_to_be() {
    // `ANTHROPIC_API_KEY` has no browser flow, so reserving the hint row for
    // it would draw a blank interior row and float the card a row too high.
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    assert_eq!(e.rows(), 3, "border, input, border");
    assert_eq!(
        e.card(60).iter().map(|c| c.row).max(),
        Some(2),
        "nothing may be drawn below the bottom border"
    );
    e.set_waiting(true);
    assert_eq!(e.rows(), 4, "the hint needs an interior row of its own");
    assert_eq!(e.card(60).iter().map(|c| c.row).max(), Some(3));
}

#[test]
fn typing_clears_the_waiting_state() {
    // Once the user starts pasting, the card should stop claiming to wait.
    let mut e = KeyEntry::new("OPENROUTER_API_KEY".into());
    e.set_waiting(true);
    typed(&mut e, "s");
    let drawn: String = e.card(60).iter().map(|c| c.c).collect();
    assert!(!drawn.contains("waiting for browser"), "{drawn}");
}

#[test]
fn a_narrow_card_keeps_the_variable_name_over_the_word_paste() {
    // `titled_card` silently clips its legend at `cols - 2`, head-first — so
    // without `fit_legend`'s tail-preserving truncation, a narrow pane would
    // show "paste ANTHROPIC_A" and lose the very name the prompt exists to
    // convey. The variable name must survive in preference to the word
    // "paste".
    let e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    let cells = e.card(20);
    let legend: String = cells.iter().filter(|c| c.row == 0).map(|c| c.c).collect();
    assert!(
        legend.contains("API_KEY"),
        "the variable name must survive a narrow legend: {legend:?}"
    );
    assert!(
        !legend.contains("paste"),
        "the word \"paste\" should be dropped before the variable name is: {legend:?}"
    );
}
