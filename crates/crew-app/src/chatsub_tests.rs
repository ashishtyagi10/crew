use super::*;
use crate::chatmsgs::{card_lines, View};

const LONG: &str = "one\ntwo\nthree\nfour\nfive";

fn reply(sender: &str, meta: &str, text: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: meta.into(),
        usage: None,
        expanded: false,
    }
}

fn line(l: &crate::chatbody::CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

#[test]
fn the_broker_decides_which_cards_are_sections() {
    assert!(is_section(&reply(
        "coder \u{2192} user",
        "sub \u{00b7} 3.2s",
        "x"
    )));
    // As stamped on a card that went through a background task.
    assert!(is_section(&reply(
        "coder \u{2192} user",
        "task:7 \u{00b7} sub \u{00b7} 3.2s",
        "x"
    )));
    // The one reply of an ordinary turn has the same SHAPE and is not one.
    assert!(!is_section(&reply("coder \u{2192} user", "3.2s", "x")));
    assert!(!is_section(&reply("coder \u{2192} user", "", "x")));
}

/// The fan-out's flood: every agent's answer arrives as its own section,
/// collapsed to a line you can skim.
#[test]
fn a_subagent_section_lands_collapsed() {
    let m = reply("code-analyst \u{2192} user", "sub \u{00b7} 3.2s", LONG);
    let lines = card_lines(&[&m], 40, 0, View::default());
    assert_eq!(
        lines.len(),
        2,
        "header + one clamped body line, got: {:?}",
        lines.iter().map(line).collect::<Vec<_>>()
    );
    let body = line(&lines[1]);
    assert!(
        body.starts_with(" one"),
        "the first line is the preview: {body}"
    );
    assert!(
        body.contains("\u{2026} +4"),
        "and it says how much is under it: {body}"
    );
}

#[test]
fn clicking_it_open_shows_the_whole_answer() {
    let mut m = reply("code-analyst \u{2192} user", "sub \u{00b7} 3.2s", LONG);
    m.expanded = true;
    let lines = card_lines(&[&m], 40, 0, View::default());
    assert_eq!(lines.len(), 6, "header + all five body lines");
}

/// A fold with no way to open it is a card that ate its own content — the
/// invariant `chatfold::foldable` exists for.
#[test]
fn a_collapsed_section_is_always_toggleable() {
    let m = reply("code-analyst \u{2192} user", "sub \u{00b7} 3.2s", LONG);
    assert!(crate::chatfold::folded(&m, 5));
    assert!(crate::chatfold::foldable(&m, 40, View::default()));
}

/// The error the fan reports for an agent that failed is one line, so it
/// stays on screen: a section folds what is long, it does not hide what is
/// short.
#[test]
fn a_one_line_section_is_not_folded_into_nothing() {
    let m = reply(
        "opencode \u{2192} user",
        "sub",
        "[error] opencode: timed out after 180s",
    );
    let lines = card_lines(&[&m], 60, 0, View::default());
    assert_eq!(lines.len(), 2, "header + the error itself");
    assert!(!line(&lines[1]).contains('\u{2026}'), "nothing was hidden");
}

/// The mark is for the renderer; a saved transcript keeps only the latency.
#[test]
fn the_mark_never_reaches_an_exported_transcript() {
    assert_eq!(
        crate::chattime::strip_task_tag("task:7 \u{00b7} sub \u{00b7} 3.2s"),
        "3.2s"
    );
    assert_eq!(crate::chattime::strip_task_tag("sub \u{00b7} 3.2s"), "3.2s");
}

/// A fan's LIVE card is a section from its first fragment — the pane is at
/// its noisiest while eight agents are typing, which is exactly when the
/// settled card's mark does not exist yet.
#[test]
fn a_marked_delta_opens_a_folded_live_card() {
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    let mut p = crate::chat::ChatPane::new(plugin, "crew".into());
    p.absorb_delta("code-analyst".into(), LONG.into(), true);
    let card = p.streaming.last().expect("a live card opened");
    assert!(is_section(card), "meta carries the mark: {:?}", card.meta);

    let view = crate::chatmsgs::View {
        streaming_from: 0,
        ..View::default()
    };
    let lines = card_lines(&[card], 40, 0, view);
    // Header + the clamped first line (the live caret rides that line).
    assert_eq!(
        lines.len(),
        2,
        "the live card folds too: {:?}",
        lines.iter().map(line).collect::<Vec<_>>()
    );
}

/// An ordinary turn's live card is untouched: one agent typing IS the answer
/// you are waiting for.
#[test]
fn an_unmarked_delta_still_streams_in_full() {
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    let mut p = crate::chat::ChatPane::new(plugin, "crew".into());
    p.absorb_delta("coder".into(), LONG.into(), false);
    let card = p.streaming.last().expect("a live card opened");
    assert!(!is_section(card));
    let view = crate::chatmsgs::View {
        streaming_from: 0,
        ..View::default()
    };
    assert_eq!(card_lines(&[card], 40, 0, view).len(), 6);
}

/// A folded section's LIVE thought keeps its head row and nothing else, so
/// the machinery never reads louder than the voice it sits over.
#[test]
fn a_folded_sections_live_thought_is_its_head_row_only() {
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    let mut p = crate::chat::ChatPane::new(plugin, "crew".into());
    p.absorb_delta("code-analyst".into(), LONG.into(), true);
    p.thoughts.absorb(
        "code-analyst",
        "first I check the focus logic\nthen the glance card\nthen the tests",
        crate::chattime::unix_now_ms(),
    );
    let card = p.streaming.last().unwrap();
    let view = crate::chatmsgs::View {
        streaming_from: 0,
        thoughts: &p.thoughts,
        ..View::default()
    };
    let rows = crate::chatthoughtseat::above(view, card, true, 0, 40);
    assert_eq!(rows.len(), 1, "head only: {:?}", rows.len());
    let head: String = rows[0].iter().map(|c| c.c).collect();
    assert!(head.contains("thinking"), "{head}");
}
