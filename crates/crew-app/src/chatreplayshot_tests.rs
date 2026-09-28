//! Replay a recorded broker session into a real chat pane and shoot it —
//! what the smith pane SHOWS for a turn, through the pane's own event path
//! (`ChatPane::poll`), not a hand-built transcript. A fake broker `cat`s the
//! recorded JSON lines; `CREW_REPLAY_CUT` stops the stream partway, for the
//! pane as it looks mid-turn.
//!
//! `#[ignore]`d (needs a GPU adapter, a recording, writes PNGs):
//! `CREW_REPLAY=<events.jsonl> CREW_REPLAY_ASK="<what was typed>"
//!  [CREW_REPLAY_CUT=<lines>] CREW_SHOT_DIR=<dir>
//!  cargo test -p crew-app --bin crew replay_shot -- --ignored`
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crew_plugin::Plugin;
use std::time::{Duration, Instant};

#[test]
#[ignore = "needs a GPU adapter and a recording; writes PNGs"]
fn replay_shot() {
    let _g = crate::app::theme_test_guard();
    let Ok(path) = std::env::var("CREW_REPLAY") else {
        eprintln!("set CREW_REPLAY to a recorded events file — skipping");
        return;
    };
    let text = std::fs::read_to_string(&path).expect("recording");
    let cut = std::env::var("CREW_REPLAY_CUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);
    let lines: Vec<&str> = text.lines().take(cut).collect();
    let tmp = std::env::temp_dir().join(format!("crew-replay-{}.jsonl", std::process::id()));
    std::fs::write(&tmp, lines.join("\n") + "\n").unwrap();
    let script = format!("cat '{}'; sleep 30", tmp.display());
    let plugin = Plugin::spawn("sh", &["-c".to_string(), script]).unwrap();
    let mut pane = ChatPane::new(plugin, "crew".into());
    let ask = std::env::var("CREW_REPLAY_ASK").unwrap_or_default();
    if !ask.is_empty() {
        pane.messages.push(Message {
            sender: "user".into(),
            text: ask.clone(),
            ts: String::new(),
            meta: String::new(),
            usage: None,
            expanded: false,
        });
        pane.submit_command(ask);
    }
    // Drain until the stream goes quiet, then let the clock-driven reveal
    // finish, so the shot is the settled frame of that moment.
    let start = Instant::now();
    let mut quiet = Instant::now();
    while start.elapsed() < Duration::from_secs(10) && quiet.elapsed() < Duration::from_millis(600)
    {
        if pane.poll().changed {
            quiet = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    std::thread::sleep(Duration::from_millis(1500));
    pane.poll();
    let name = std::env::var("CREW_REPLAY_NAME").unwrap_or_else(|_| "replay".into());
    let shot =
        crate::shotgpu_tests::shot_at(&name, 760, 760, 13.0, "crew", |cols, rows, aspect| {
            crate::chatview::art(&pane, cols, rows, aspect)
        });
    let _ = std::fs::remove_file(&tmp);
    if shot.is_none() {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
    }
}
