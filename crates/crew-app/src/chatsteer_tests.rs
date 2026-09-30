use super::*;
use crate::chatkeys::ChatInput;
use crew_plugin::Plugin;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

const TEXT: &str = "also check the tests";

fn record_file() -> PathBuf {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    std::env::temp_dir().join(format!(
        "crew-chatsteer-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}

/// A connected pane mid-turn (a send is out) whose broker first prints
/// `lines`, then writes every command it receives to `file`. What it printed
/// waits in the pipe until the test polls.
fn busy_pane(lines: &[&str], file: &Path) -> ChatPane {
    let prints: String = lines
        .iter()
        .map(|l| format!("printf '%s\\n' '{l}'; "))
        .collect();
    let script = format!("{prints}cat > '{}'", file.display());
    let plugin = Plugin::spawn("sh", &["-c".to_string(), script]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;
    p.awaiting = true;
    p
}

/// Everything the pane sent, in order: a marker goes last and the pipe keeps
/// order, so once it is in the file everything before it is too.
fn sent(p: &mut ChatPane, file: &Path) -> Vec<String> {
    let end = PluginCommand::Subscribe {
        channel: "end".into(),
    };
    p.plugin.send(&end).unwrap();
    let marker = serde_json::to_string(&end).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let s = std::fs::read_to_string(file).unwrap_or_default();
        if s.contains(&marker) {
            let _ = std::fs::remove_file(file);
            return s
                .lines()
                .filter(|l| *l != marker)
                .map(String::from)
                .collect();
        }
        assert!(Instant::now() < deadline, "the broker never got the marker");
        std::thread::yield_now();
    }
}

fn type_and_enter(p: &mut ChatPane, text: &str) {
    p.input = text.into();
    p.on_input(ChatInput::Enter, &std::env::temp_dir());
}

fn steer_line(text: &str) -> String {
    serde_json::to_string(&PluginCommand::Steer {
        channel: "crew".into(),
        text: text.into(),
    })
    .unwrap()
}

fn send_line(text: &str) -> String {
    serde_json::to_string(&PluginCommand::Send {
        channel: "crew".into(),
        text: text.into(),
    })
    .unwrap()
}

fn poll_until(p: &mut ChatPane, done: impl Fn(&ChatPane) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done(p) {
        p.poll();
        assert!(Instant::now() < deadline, "the pane never got there");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn plain_words_steer_and_the_things_the_broker_routes_do_not() {
    assert!(steers(TEXT));
    assert!(steers("  no, the other file"));
    for routed in [
        "/diff",
        "/model all x",
        "#always use tabs",
        "@claude look",
        "",
        "  ",
    ] {
        assert!(!steers(routed), "{routed:?}");
    }
}

#[test]
fn the_note_quotes_the_first_line_and_clips_a_long_one() {
    assert_eq!(
        joined_note("no, the other file\n--- file ---\nbody"),
        format!("{JOINED}: \u{201c}no, the other file\u{201d}")
    );
    let long = "x".repeat(QUOTE + 5);
    let note = joined_note(&long);
    assert!(note.ends_with("x\u{2026}\u{201d}"), "{note}");
    assert_eq!(note.matches('x').count(), QUOTE);
}

#[test]
fn a_plain_message_typed_while_busy_is_queued_and_offered() {
    let file = record_file();
    let mut p = busy_pane(&[], &file);
    type_and_enter(&mut p, TEXT);
    assert_eq!(p.queued, [TEXT], "held as before");
    assert_eq!(
        sent(&mut p, &file),
        [steer_line(TEXT)],
        "and offered, not sent"
    );
    let echo = p.messages.last().unwrap();
    assert_eq!((echo.sender.as_str(), echo.text.as_str()), ("user", TEXT));
}

#[test]
fn a_slash_command_typed_while_busy_is_only_queued() {
    let file = record_file();
    let mut p = busy_pane(&[], &file);
    type_and_enter(&mut p, "/diff");
    assert_eq!(p.queued, ["/diff"]);
    assert!(sent(&mut p, &file).is_empty());
}

#[test]
fn nothing_is_offered_on_a_dead_connection_or_while_a_plan_waits() {
    let file = record_file();
    let mut p = busy_pane(&[], &file);
    p.connected = false;
    type_and_enter(&mut p, TEXT);
    p.connected = true;
    p.plan_pending = true; // text typed: Enter sends it, the plan stays
    type_and_enter(&mut p, "yes");
    assert_eq!(p.queued, [TEXT, "yes"]);
    assert!(sent(&mut p, &file).is_empty());
}

#[test]
fn idle_sends_at_once_as_before() {
    let file = record_file();
    let mut p = busy_pane(&[], &file);
    p.awaiting = false;
    type_and_enter(&mut p, TEXT);
    assert!(p.queued.is_empty());
    assert_eq!(sent(&mut p, &file), [send_line(TEXT)]);
}

#[test]
fn steered_takes_exactly_the_matching_queued_copy_and_notes_it() {
    let file = record_file();
    let steered = r#"{"type":"steered","channel":"crew","text":"a"}"#;
    let mut p = busy_pane(&[steered], &file);
    p.queued.extend(["a".to_string(), "b".into(), "a".into()]);
    poll_until(&mut p, |p| p.queued.len() == 2);
    assert_eq!(p.queued, ["b", "a"], "the first equal one, and only it");
    let note = p.messages.last().unwrap();
    assert_eq!(note.sender, "agent smith");
    assert_eq!(note.text, joined_note("a"));
    assert!(p.is_busy(), "the turn runs on");
    assert!(
        sent(&mut p, &file).is_empty(),
        "nothing flushed, nothing re-sent"
    );
}

#[test]
fn an_untaken_steer_still_flushes_when_the_turn_settles() {
    let file = record_file();
    let reply = r#"{"type":"message","channel":"crew","sender":"planner","text":"done","ts":""}"#;
    let mut p = busy_pane(&[reply], &file);
    type_and_enter(&mut p, TEXT);
    poll_until(&mut p, |p| p.queued.is_empty());
    assert_eq!(sent(&mut p, &file), [steer_line(TEXT), send_line(TEXT)]);
    assert!(
        !p.messages.iter().any(|m| m.text.starts_with(JOINED)),
        "never taken, never said to have joined"
    );
}
