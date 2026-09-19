//! Off-screen render of a FANNED turn — the shape that sent eleven full
//! answers into one pane — so the folded sections can be looked at rather
//! than counted in `CellView`s.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew fan_shot -- --ignored`
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crate::shotgpu_tests::shot_at;
use crew_plugin::Plugin;

const H: u32 = 760;

/// One agent's fanned answer: long enough that unfolded it owns the screen.
fn answer(agent: &str, task: u64, body: &str) -> Message {
    Message {
        sender: format!("{agent} \u{2192} user"),
        text: body.into(),
        ts: String::new(),
        meta: format!("task:{task} \u{00b7} sub \u{00b7} 3.2s"),
        usage: Some((1840, 420, 9100)),
        expanded: false,
    }
}

fn plain(sender: &str, text: &str, meta: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: meta.into(),
        usage: None,
        expanded: false,
    }
}

const BODY: &str = "Suggested first pick: item 1. It finishes a goal you already \
                    started, touches only focus logic and one glance card, and is \
                    easily tested with the existing TailWatch tests.\n\n\
                    - the advance-on-answer queue is half built already\n\
                    - the glance card reads the same set\n\
                    - nothing else in the pane depends on either\n\n\
                    Item 4 is the cheapest, but it buys the least.";

fn fanned_pane() -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;
    p.messages = vec![
        plain("user", "ask everyone: what should we build next?", ""),
        plain(
            "agent smith",
            "fanning out to 8 agents in parallel\u{2026}",
            "task:7",
        ),
        answer("code-analyst", 7, BODY),
        answer("editor", 7, BODY),
        answer("writer", 7, BODY),
        answer("communicator", 7, BODY),
        answer("analyst", 7, BODY),
        answer("documenter", 7, BODY),
        answer("software-engineer", 7, BODY),
        plain(
            "opencode \u{2192} user",
            "[error] opencode: timed out after 180s",
            "task:7 \u{00b7} sub",
        ),
        plain(
            "agent smith",
            "fan done \u{2014} 7 of 8 replied \u{2225} analyst 2.2s \u{00b7} \
             editor 3.9s \u{00b7} writer 34.7s \u{00b7} 2274 tok",
            "task:7",
        ),
    ];
    p.tokens = 2_274;
    p.turns = 1;
    p.cwd = Some("~/code/crew".into());
    p
}

fn fan_shot(name: &str, w: u32, open: bool) -> Option<Vec<u8>> {
    let mut pane = fanned_pane();
    if open {
        pane.messages[2].expanded = true; // the one you clicked
    }
    shot_at(name, w, H, 13.0, "crew", |cols, rows, aspect| {
        crate::chatview::art(&pane, cols, rows, aspect)
    })
}

/// The turn from the bug report, at a half tile and at a full window: every
/// agent that answered is on screen at once, with the `fan done` line — the
/// line you actually wanted — still under them.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn fan_shot_sections_are_collapsed() {
    let _g = crate::app::theme_test_guard();
    for (name, w) in [("fan-half", 700), ("fan-full", 1180)] {
        let Some(px) = fan_shot(name, w, false) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 4000, "{name} drew");
    }
}

/// …and the same turn with one section clicked open, which is the only way
/// to read an answer now: the one you asked for, in full, with the rest
/// still one line each.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn fan_shot_one_section_open() {
    let _g = crate::app::theme_test_guard();
    let Some(px) = fan_shot("fan-open", 700, true) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(crate::shotgpu_tests::ink(&px) > 4000, "fan-open drew");
}
