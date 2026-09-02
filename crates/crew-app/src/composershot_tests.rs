//! Off-screen render of what a crew pane shows before and around its
//! transcript: the empty state in its three shapes, the auto-folded system
//! card, the folded tool card and the compaction marker inside a transcript.
//! The two cards that sit over the composer — Cmd+F find and Ctrl+R history
//! search — are `composercards_tests`, on the same fixture.
//!
//! `chatshot` renders a live conversation; these are the states a session
//! passes through on its way in and out of one, and none had been in a frame.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew composer_shot -- --ignored --nocapture`
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crate::goalshot_tests::dump;
use crate::shotgpu_tests::shot_at;
use crew_plugin::{AgentInfo, Plugin};

fn msg(sender: &str, text: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

pub(crate) fn pane() -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;
    p.agents = vec![
        AgentInfo {
            name: "smith".into(),
            role: "lead".into(),
            model: "claude-opus-5".into(),
        },
        AgentInfo {
            name: "scout".into(),
            role: "search".into(),
            model: "claude-haiku-4-5".into(),
        },
    ];
    p
}

/// A transcript with every card shape that folds: `/doctor` in the system
/// voice, a tool call under an agent's name, and the compaction marker.
pub(crate) fn folded_pane() -> ChatPane {
    let mut p = pane();
    p.messages = vec![
        crate::chatcompact::marker(12),
        msg("user", "/doctor"),
        msg(
            "agent smith",
            "provider: anthropic (claude-opus-5) · key from env\n\
             mcp: 2 servers, 14 tools\n\
             skills: 6 loaded from ~/.crew/skills\n\
             integrations: github, linear\n\
             sidecar: not configured\n\
             sandbox: read-only, cwd ~/code/crew\n\
             clock: 3 standing intents, next 09:00 tomorrow\n\
             channels: loopback, telegram (@crew_bot)",
        ),
        msg("user", "@scout what changed in plot/ since v0.19.60?"),
        msg(
            "scout",
            "[tool] sys:run \u{2713} 1.2s\n\
             $ git log --oneline v0.19.60..HEAD -- crates/crew-app/src/plot\n\
             3f1c2aa feat(plot): sdf arcs\n\
             9be0d11 fix(plot): canvas follows the device pixel\n\
             71a0c9e feat(plot): gantt rule at now",
        ),
        msg(
            "scout",
            "Three commits: the SDF arcs, the device-pixel canvas, and the gantt's \
             now-rule. The second is the one that changed how every dial looks.",
        ),
    ];
    p
}

fn pane_shot(name: &str, p: &ChatPane, w: u32, h: u32) -> Option<Vec<String>> {
    let mut dumped = Vec::new();
    shot_at(
        &format!("composer-{name}"),
        w,
        h,
        13.0,
        "crew",
        |cols, rows, aspect| {
            let (cells, paint) = crate::chatview::art(p, cols, rows, aspect);
            dumped = dump(&cells, cols, rows);
            eprintln!("--- composer-{name} {cols}x{rows}");
            for l in &dumped {
                eprintln!("|{l}");
            }
            (cells, paint)
        },
    )?;
    Some(dumped)
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn composer_shot_folds() {
    let _g = crate::app::theme_test_guard();
    let p = folded_pane();
    for (name, w, h) in [("folds", 760, 420), ("folds-narrow", 380, 420)] {
        let Some(rows) = pane_shot(name, &p, w, h) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(rows.iter().any(|r| r.contains("\u{2026} +7")), "{rows:?}");
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn composer_shot_empty_states() {
    let _g = crate::app::theme_test_guard();
    let fresh = pane();
    let mut connecting = pane();
    connecting.connected = false;
    let mut lonely = pane();
    lonely.agents.clear();
    for (name, p, w) in [
        ("empty", &fresh, 700),
        ("empty-narrow", &fresh, 380),
        ("connecting", &connecting, 700),
        ("no-agents", &lonely, 700),
        ("no-agents-narrow", &lonely, 380),
    ] {
        if pane_shot(name, p, w, 260).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
}
