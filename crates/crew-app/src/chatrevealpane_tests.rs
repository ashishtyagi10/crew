use crate::chat::tests::pane;
use crate::chat::ChatPane;
use crate::chatmsgs::card_lines;
use crate::chatmsgs::tests::msg;
use crate::motion::set_level;
use crate::motion::MotionLevel::{Full, Off};

const N: usize = 200;

/// A pane with one provisional 200-`x` card whose delta landed at `at`.
fn streaming(at: u64) -> ChatPane {
    let mut p = pane();
    p.streaming.push(msg("coder", &"x".repeat(N)));
    p.note_delta("coder", 0, N, at);
    p
}

/// How many `x` the card view draws at `now`, plus its last body line.
fn drawn(p: &ChatPane, now: u64) -> (usize, String) {
    let lines = card_lines(&p.visible_messages(), 80, now, p.view());
    let xs = lines.iter().flatten().filter(|c| c.c == 'x').count();
    let last: String = lines.last().unwrap().iter().map(|c| c.c).collect();
    (xs, last)
}

#[test]
fn a_streaming_card_types_out_instead_of_landing_whole() {
    let _g = crate::app::motion_test_guard();
    set_level(Full);
    let p = streaming(1_000);
    let (at_zero, _) = drawn(&p, 1_000);
    assert!(
        at_zero < N,
        "t=0 after a 200-char delta: {at_zero} of {N} shown"
    );
    let (mid, last) = drawn(&p, 1_100);
    assert!(mid > at_zero && mid < N, "typing at +100ms: {mid}");
    // The caret sits right after the last revealed character.
    assert!(
        last.ends_with("x\u{258c}"),
        "caret after the typed text: {last:?}"
    );
    assert_eq!(drawn(&p, 4_000).0, N, "all of it by +3s");
}

#[test]
fn motion_off_shows_the_whole_card_at_once() {
    let _g = crate::app::motion_test_guard();
    set_level(Off);
    let p = streaming(1_000);
    assert_eq!(drawn(&p, 1_000).0, N, "Off: every character at t=0");
    assert!(!p.is_revealing(), "and nothing left to animate");
    set_level(Full);
}

#[test]
fn settling_continues_from_what_was_visible_without_a_snap_back() {
    let _g = crate::app::motion_test_guard();
    set_level(Full);
    let mut p = streaming(1_000);
    let (seen, _) = drawn(&p, 1_200);
    assert!(seen > 0 && seen < N, "mid-stream: {seen}");
    // The settled `Message` lands (stamped inside the fade window, so an
    // ordinary card would still be fading in).
    p.note_settle("coder \u{2192} user", "1150", N, 1_200);
    p.settle_stream("coder \u{2192} user");
    let mut m = msg("coder \u{2192} user", &"x".repeat(N));
    m.ts = "1150".into();
    p.push_capped(m);
    assert!(p.streaming.is_empty() && p.messages.len() == 1);
    let lines = card_lines(&p.visible_messages(), 80, 1_200, p.view());
    let xs = lines.iter().flatten().filter(|c| c.c == 'x').count();
    assert_eq!(
        xs, seen,
        "the settled card picks up exactly where the stream was"
    );
    let done = 1_200 + crate::chatreveal::CATCHUP_MS;
    assert_eq!(
        drawn(&p, done).0,
        N,
        "and finishes within the catch-up window"
    );
    // No fade-in re-fires on a card that was already on screen: the gutter
    // cell is at full ink now, exactly as it will be long after the fade
    // window — and it WOULD be blended toward the page without the state.
    let settled_ink = card_lines(&p.visible_messages(), 80, 99_000, p.view())[0][0].fg;
    assert_eq!(
        lines[0][0].fg, settled_ink,
        "the settled card must not fade in again"
    );
    p.reveal_all();
    let faded = card_lines(&p.visible_messages(), 80, 1_200, p.view())[0][0].fg;
    assert_ne!(
        faded, settled_ink,
        "fixture: an un-streamed card at this stamp fades"
    );
}

#[test]
fn a_shorter_settled_text_shows_whole_at_once() {
    let _g = crate::app::motion_test_guard();
    set_level(Full);
    let mut p = streaming(1_000);
    let (seen, _) = drawn(&p, 1_500);
    assert!(
        seen > 10,
        "enough on screen to be ahead of a short reply: {seen}"
    );
    p.note_settle("coder", "9", 5, 1_500);
    p.settle_stream("coder");
    let mut m = msg("coder", "xxxxx");
    m.ts = "9".into();
    p.push_capped(m);
    assert_eq!(drawn(&p, 1_500).0, 5);
}

#[test]
fn a_card_still_typing_keeps_the_frames_coming_after_the_pane_goes_idle() {
    let _g = crate::app::motion_test_guard();
    set_level(Full);
    let mut p = pane();
    p.absorb_delta("coder".into(), "x".repeat(N)); // the real clock: just now
    assert!(
        !p.is_busy(),
        "fixture: no hop is open, so `pane_busy` is false"
    );
    assert!(p.is_revealing(), "the reveal alone must schedule redraws");
    p.reveal_all();
    assert!(!p.is_revealing());
}

#[test]
fn the_newest_characters_are_dimmer_than_the_settled_ones() {
    // The theme guard IS the motion lock (`motion_test_guard` aliases it),
    // and this test reads `theme().ink` — so it is the one to take.
    let (_g, _off) = (crate::app::theme_test_guard(), crate::glyphs::force(false));
    set_level(Full);
    let p = streaming(1_000);
    let lines = card_lines(&p.visible_messages(), 80, 1_400, p.view());
    let xs: Vec<_> = lines.iter().flatten().filter(|c| c.c == 'x').collect();
    assert!(xs.len() > 20, "{}", xs.len());
    let ink = crew_theme::theme().ink;
    assert_eq!(xs[0].fg, ink, "long-revealed text is at full ink");
    assert_ne!(
        xs[xs.len() - 1].fg,
        ink,
        "the just-typed character is still ramping up"
    );
}

#[test]
fn a_turn_ending_without_a_message_drops_the_provisional_state() {
    let mut p = streaming(1_000);
    assert_eq!(p.reveals.len(), 1);
    p.flush_active_hops();
    assert!(p.reveals.is_empty() && p.streaming.is_empty());
}
