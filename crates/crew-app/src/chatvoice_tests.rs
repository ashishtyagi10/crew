//! The broker's bracketed card kinds are machinery, never drawn.
use super::*;
use crate::chatmsgs::tests::msg;
use crate::chatmsgs::{card_lines, View};

fn body(m: &Message) -> String {
    card_lines(&[m], 60, 0, View::default())[1..]
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The fan's report for an agent that failed: a `✗` where the bracketed
/// word was, in the ink a failed tool call's `✗` wears.
#[test]
fn an_error_card_leads_with_a_cross_in_the_removed_ink() {
    let _g = crate::app::theme_test_guard();
    let m = msg(
        "opencode \u{2192} user",
        "[error] opencode: timed out after 180s",
    );
    let (fg, text) = body_voice(&m);
    assert_eq!(text, "\u{2717} opencode: timed out after 180s");
    let removed = crate::chatink::token_fg(crate::md::syntax::Token::Removed);
    assert_eq!(fg, removed);
    assert_ne!(fg, crew_theme::theme().ink, "it does not read as a reply");
    let drawn = body(&m);
    assert!(!drawn.contains("[error]"), "{drawn:?}");
    assert!(drawn.contains("\u{2717} opencode: timed out"), "{drawn:?}");
}

/// Only a LEADING marker is machinery: an agent that writes the word in a
/// sentence is quoted as written, in ink.
#[test]
fn a_marker_mid_sentence_is_left_alone() {
    let _g = crate::app::theme_test_guard();
    let m = msg("coder", "grep for [error] in the log");
    let (fg, text) = body_voice(&m);
    assert_eq!(text, "grep for [error] in the log");
    assert_eq!(fg, crew_theme::theme().ink);
}

/// The tool marker still goes, and still mutes the card.
#[test]
fn a_tool_card_is_stripped_and_muted() {
    let _g = crate::app::theme_test_guard();
    let m = msg("coder", "[tool] fs:read src/main.rs");
    let (fg, text) = body_voice(&m);
    assert_eq!(text, "fs:read src/main.rs");
    assert_eq!(fg, crew_theme::theme().text_muted);
}
