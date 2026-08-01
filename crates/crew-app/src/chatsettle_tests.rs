use crate::chat::tests::pane;

const LONG: &str = "one\ntwo\nthree\nfour\nfive"; // > chatfold::FOLD_LINES

// --- S2: a streaming system card clicked open must stay open on settle ---

#[test]
fn a_clicked_open_streaming_card_settles_expanded() {
    let mut p = pane();
    p.absorb_delta("crew".into(), LONG.into());
    p.streaming[0].expanded = true; // the click (see `chatfold`)
    p.absorb_message("crew".into(), LONG.into(), "1".into(), String::new());
    assert!(p.streaming.is_empty(), "the provisional card is replaced");
    assert!(
        p.messages[0].expanded,
        "the click must survive the stream settling into a real Message"
    );
}

#[test]
fn an_unclicked_streaming_card_settles_folded() {
    let mut p = pane();
    p.absorb_delta("crew".into(), LONG.into());
    p.absorb_message("crew".into(), LONG.into(), "1".into(), String::new());
    assert!(
        !p.messages[0].expanded,
        "no click, so it settles auto-folded"
    );
}

#[test]
fn an_arrow_form_sender_settles_the_bare_name_card_and_keeps_its_click() {
    // `stream_key` normalizes both sides: the relay's `"crew → user"` reply
    // replaces (and inherits from) a card streamed under the bare `"crew"`.
    let mut p = pane();
    p.absorb_delta("crew".into(), LONG.into());
    p.streaming[0].expanded = true;
    p.absorb_message(
        "crew \u{2192} user".into(),
        LONG.into(),
        "1".into(),
        String::new(),
    );
    assert!(p.streaming.is_empty());
    assert!(p.messages[0].expanded);
}

// --- S4: the usage stash drains only onto the sender its stat named ---

#[test]
fn usage_stash_waits_for_the_sender_the_stat_named() {
    let mut p = pane();
    p.absorb_stats(950, "coder".into(), 1_200, 900, 900, 50, 12_000);
    // An interleaved message from another sender: no trailer, stash kept —
    // the matching reply may be the very next event.
    p.absorb_message("crew".into(), "note".into(), "1".into(), String::new());
    assert_eq!(p.messages[0].usage, None, "wrong sender wears no trailer");
    assert!(
        p.pending_reply_usage.is_some(),
        "a mismatch must keep the stash, not drop it"
    );
    // The reply the stat named (arrow form) still gets its trailer.
    p.absorb_message(
        "coder \u{2192} user".into(),
        "done".into(),
        "2".into(),
        String::new(),
    );
    assert_eq!(p.messages[1].usage, Some((900, 50, 12_000)));
    assert_eq!(p.pending_reply_usage, None, "a match drains the stash");
}
