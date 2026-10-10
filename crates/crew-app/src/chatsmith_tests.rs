//! Agent smith's two voices. His bare name is the broker's chrome — the
//! routing line, the plan line, a snapshot note — drawn muted behind a dotted
//! gutter and folded to three lines. `agent smith → user` is him ANSWERING
//! (the broker's `relay::SMITH_ANSWERS`: the swarm's closing answer, the
//! recall readout), and it draws as any agent's reply: solid gutter, his
//! roster colour, ink, never folded.
use super::*;
use crate::chatfold::{foldable, folded};
use crate::chatmsgs::tests::{card_row, msg};
use crate::chatmsgs::{card_lines, message_cells, View};

const ANSWER: &str = "agent smith \u{2192} user";
const LONG: &str = "one\ntwo\nthree\nfour\nfive"; // > chatfold::FOLD_LINES

#[test]
fn smiths_answer_is_a_reply_and_his_bare_name_is_still_chrome() {
    assert!(!is_system_voice(ANSWER), "an answer is not telemetry");
    assert!(is_system_voice("agent smith"), "his chrome still is");
    let (answer, chrome) = (msg(ANSWER, LONG), msg("agent smith", LONG));
    let v = View::default();
    assert!(!folded(&answer, 5), "an answer never auto-folds");
    assert!(
        !foldable(&answer, 40, v),
        "so there is nothing to click open"
    );
    assert!(folded(&chrome, 5) && foldable(&chrome, 40, v));
    // Header + all five lines, against header + first line + `… +4`.
    assert_eq!(card_lines(&[&answer], 40, 0, v).len(), 6);
    assert_eq!(card_lines(&[&chrome], 40, 0, v).len(), 2);
}

#[test]
fn smiths_answer_wears_his_roster_colour_his_badge_and_ink() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    // The hue his pane legend and his thought fold already wear.
    let smith = crate::chatroster::agent_color("agent smith");
    assert_ne!(smith, t.text_muted);
    let answer = header_line(&msg(ANSWER, "x"), 0, None);
    assert_eq!(answer[0].c, GUTTER, "the solid gutter");
    assert_eq!(answer[0].fg, smith, "in his colour");
    let badge = crate::segment::inked(smith).1;
    assert_eq!(answer[3].bg, Some(badge), "his name on its badge");
    let chrome = header_line(&msg("agent smith", "x"), 0, None);
    assert_eq!(chrome[0].c, '\u{2506}', "chrome keeps the dotted gutter");
    assert_eq!(chrome[0].fg, t.text_muted);
    let ink = |sender: &str| crate::chatvoice::body_voice(&msg(sender, "x")).0;
    assert_eq!(ink(ANSWER), t.ink, "an answer speaks in ink");
    assert_eq!(ink("agent smith"), t.text_muted);
}

#[test]
fn smiths_answer_header_reads_from_him_to_you() {
    let cells = message_cells(&[&msg(ANSWER, "hello")], 40, 10, 0, 0, View::default());
    assert_eq!(
        card_row(&cells, 0),
        format!("{GUTTER}\u{2590} agent smith \u{258c} \u{2192} user")
    );
}

#[test]
fn smiths_streamed_answer_is_a_reply_card_and_settles_into_his_message() {
    let mut p = crate::chat::tests::pane();
    p.absorb_delta(ANSWER.into(), "Both ".into(), false);
    p.absorb_delta(ANSWER.into(), "agree.".into(), false);
    assert_eq!(p.streaming.len(), 1, "one live card");
    assert!(
        !is_system_voice(&p.streaming[0].sender),
        "drawn as a reply live"
    );
    p.absorb_message(
        ANSWER.into(),
        "Both agree.".into(),
        "1".into(),
        String::new(),
    );
    assert!(
        p.streaming.is_empty(),
        "the answer takes the live card's place"
    );
    assert_eq!(p.messages.last().map(|m| m.sender.as_str()), Some(ANSWER));
}

#[test]
fn a_closing_call_that_fails_mid_stream_still_settles_the_live_card() {
    // The failure is his chrome, and pairs with the answer's live card by
    // the name before the arrow — a half-written answer is not left behind.
    let mut p = crate::chat::tests::pane();
    p.absorb_delta(ANSWER.into(), "Both ".into(), false);
    let failed = "could not combine the workers' answers: timed out";
    p.absorb_message(
        "agent smith".into(),
        failed.into(),
        "1".into(),
        String::new(),
    );
    assert!(p.streaming.is_empty(), "the live card was left behind");
}
