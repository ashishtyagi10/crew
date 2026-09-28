use super::*;
use crew_plugin::Plugin;
use std::sync::atomic::{AtomicU32, Ordering};

#[test]
fn the_first_key_of_a_message_warms() {
    assert!(WarmLatch::default().edge(true, false, Instant::now()));
}

#[test]
fn a_second_key_inside_twenty_seconds_does_not() {
    let (mut l, t) = (WarmLatch::default(), Instant::now());
    assert!(l.edge(true, false, t));
    // The next key of the same message: not an edge at all.
    assert!(!l.edge(false, false, t + Duration::from_millis(80)));
    // A message sent, and the next one begun, inside the window: an edge,
    // but the socket the last warm opened is still in the pool.
    assert!(!l.edge(true, false, t + EVERY - Duration::from_millis(1)));
}

#[test]
fn empty_to_non_empty_after_twenty_seconds_warms_again() {
    let (mut l, t) = (WarmLatch::default(), Instant::now());
    assert!(l.edge(true, false, t));
    assert!(l.edge(true, false, t + EVERY));
    assert!(!l.edge(true, false, t + EVERY + Duration::from_secs(1)));
}

/// Emptying the composer, or a key that leaves it empty (a Backspace on
/// nothing), starts no message.
#[test]
fn only_the_filling_edge_counts() {
    let (mut l, t) = (WarmLatch::default(), Instant::now());
    assert!(!l.edge(false, true, t));
    assert!(!l.edge(true, true, t));
    assert!(l.edge(true, false, t), "the refusals above spent nothing");
}

/// A pane whose broker writes every command it receives to a file, so a
/// test can read back exactly what the pane sent, in order.
fn recording_pane() -> (ChatPane, std::path::PathBuf) {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let file = std::env::temp_dir().join(format!(
        "crew-chatwarm-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let script = format!("cat > '{}'", file.display());
    let plugin = Plugin::spawn("sh", &["-c".to_string(), script]).unwrap();
    (ChatPane::new(plugin, "crew".into()), file)
}

/// Everything `pane` has sent: a marker goes last, and the pipe keeps
/// order, so once the marker is in the file everything before it is too.
fn sent(pane: &mut ChatPane, file: &std::path::Path) -> Vec<String> {
    let end = PluginCommand::Subscribe {
        channel: "end".into(),
    };
    pane.plugin.send(&end).unwrap();
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

#[test]
fn typing_a_message_sends_one_warm_through_the_plugin() {
    let (mut pane, file) = recording_pane();
    let cwd = std::env::temp_dir();
    for c in "hi".chars() {
        pane.on_typed(ChatInput::Char(c), &cwd);
    }
    assert_eq!(pane.input, "hi");
    assert_eq!(sent(&mut pane, &file), vec![r#"{"type":"warm"}"#]);
}

#[test]
fn a_paste_into_an_empty_composer_warms_too() {
    let (mut pane, file) = recording_pane();
    pane.paste_composer("why does this fail?");
    pane.paste_composer(" and this");
    assert_eq!(sent(&mut pane, &file), vec![r#"{"type":"warm"}"#]);
}
