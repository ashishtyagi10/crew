use crew_plugin::Plugin;

use crate::chat::ChatPane;
use crate::chatkeys::ChatInput;

/// A live pane whose "broker" writes each start to a file and then idles.
fn pane(tag: &str) -> (ChatPane, std::path::PathBuf) {
    let file = std::env::temp_dir().join(format!("crew-watch-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let sh = format!("echo started >> '{}'; cat >/dev/null", file.display());
    let plugin = Plugin::spawn("sh", &["-c".to_string(), sh]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;
    (p, file)
}

/// How many times the broker has started, once `want` starts have landed.
fn starts(file: &std::path::Path, want: usize) -> usize {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let n = std::fs::read_to_string(file)
            .unwrap_or_default()
            .lines()
            .count();
        if n >= want || std::time::Instant::now() > deadline {
            return n;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn last_note(p: &ChatPane) -> String {
    p.messages
        .last()
        .map(|m| m.text.clone())
        .unwrap_or_default()
}

/// A broker that has been beating and goes silent has hung: it is restarted,
/// and the pane says why. One that never beat (an older broker) is not judged.
#[test]
fn a_broker_that_stops_beating_is_restarted_and_one_that_never_beat_is_not() {
    let (mut p, file) = pane("wedged");
    assert_eq!(starts(&file, 1), 1, "the first broker is up");
    p.watch.last_event = 1_000;
    assert!(
        p.watchdog_at(60_000).is_none(),
        "no heartbeat ever: nothing to miss"
    );
    p.watch.beats = true;
    assert!(
        p.watchdog_at(1_000 + 19_000).is_none(),
        "a beat is only 5 s late"
    );
    assert!(p.watchdog_at(1_000 + 20_001).is_some(), "four missed beats");
    assert!(
        last_note(&p).contains("stopped answering"),
        "{}",
        last_note(&p)
    );
    assert_eq!(starts(&file, 2), 2, "the broker was started again");
    let _ = std::fs::remove_file(&file);
}

/// A task with nothing new for 90 s gets a note saying what it waits on and
/// that Esc stops it — once, then again only after three more minutes.
#[test]
fn a_quiet_task_says_what_it_is_waiting_on() {
    let (mut p, _) = pane("quiet");
    p.absorb_task(1, true);
    p.active.push(crate::chatflow::ActiveAgent {
        name: "coder".into(),
        from: String::new(),
        since: std::time::Instant::now(),
        tool: Some("sys:run".into()),
    });
    p.watch.last_work = 1_000;
    let notes = |p: &ChatPane| {
        p.messages
            .iter()
            .filter(|m| m.text.contains("still working"))
            .count()
    };
    p.watchdog_at(1_000 + 89_000);
    assert_eq!(notes(&p), 0, "not yet");
    p.watchdog_at(1_000 + 90_001);
    assert_eq!(notes(&p), 1);
    let note = last_note(&p);
    assert!(
        note.contains("1m30") && note.contains("coder is waiting on sys:run"),
        "{note}"
    );
    assert!(note.contains("Esc stops it"), "{note}");
    p.watchdog_at(1_000 + 200_000);
    assert_eq!(notes(&p), 1, "not again so soon");
    p.watchdog_at(1_000 + 90_001 + 180_001);
    assert_eq!(notes(&p), 2, "and again after three minutes");
}

/// Esc arms the stop clock; a stop that has not taken in 20 s ends the
/// broker, without telling the user to send the message again.
#[test]
fn an_esc_that_does_not_take_ends_the_broker() {
    let (mut p, file) = pane("stop");
    assert_eq!(starts(&file, 1), 1, "the first broker is up");
    p.awaiting = true;
    p.on_input(ChatInput::Close, &std::env::temp_dir());
    let asked = p.watch.stop_asked.expect("Esc while busy asked for a stop");
    p.awaiting = true; // the broker never answered the /stop
    assert!(p.watchdog_at(asked + 19_000).is_none());
    assert!(p.watchdog_at(asked + 20_001).is_some());
    let note = last_note(&p);
    assert!(
        note.contains("did not stop within 20 seconds of Esc"),
        "{note}"
    );
    assert!(!note.contains("send your last message"), "{note}");
    assert!(!p.is_busy(), "and the pane is idle again");
    assert_eq!(starts(&file, 2), 2);
    let _ = std::fs::remove_file(&file);
}

/// A stop that took — the pane went idle — is forgotten.
#[test]
fn a_stop_that_took_is_forgotten() {
    let (mut p, _) = pane("took");
    p.watch.stop_asked = Some(1_000);
    assert!(p.watchdog_at(1_000 + 60_000).is_none());
    assert!(p.watch.stop_asked.is_none());
}
