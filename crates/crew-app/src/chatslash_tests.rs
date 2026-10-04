use std::time::{Duration, Instant};

use crew_plugin::{ApprovalMode, Plugin};

use crate::chat::ChatPane;
use crate::chatkeys::ChatInput;

/// A pane whose "broker" writes every command it is sent to `file`.
fn pane(tag: &str) -> (ChatPane, std::path::PathBuf) {
    let file = std::env::temp_dir().join(format!("crew-slash-{}-{tag}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let sh = format!("cat > '{}'", file.display());
    let plugin = Plugin::spawn("sh", &["-c".to_string(), sh]).unwrap();
    (ChatPane::new(plugin, "crew".into()), file)
}

fn sent(file: &std::path::Path, want: usize) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        if text.lines().count() >= want || Instant::now() > deadline {
            return text;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn enter(p: &mut ChatPane, line: &str) {
    p.input = line.into();
    p.on_input(ChatInput::Enter, &std::env::temp_dir());
}

/// `/approvals <name>` is Shift+Tab in words, other agents' names included; `/approvals`
/// alone says what is on; a name that is no mode says which ones are.
#[test]
fn slash_approvals_sets_names_and_explains_the_mode() {
    let (mut p, file) = pane("mode");
    enter(&mut p, "/approvals plan");
    assert_eq!(p.approval_mode, ApprovalMode::Plan);
    enter(&mut p, "/approvals accept-edits");
    assert_eq!(p.approval_mode, ApprovalMode::Edits);
    enter(&mut p, "/approvals YOLO");
    assert_eq!(p.approval_mode, ApprovalMode::Yolo);
    let lines = sent(&file, 3);
    assert!(
        lines.contains(r#""approval":"plan""#) && lines.contains(r#""approval":"yolo""#),
        "{lines}"
    );
    enter(&mut p, "/approvals");
    let last = p
        .messages
        .last()
        .map(|m| m.text.clone())
        .unwrap_or_default();
    assert!(last.starts_with("approval mode: yolo"), "{last}");
    enter(&mut p, "/approvals careful");
    let last = p
        .messages
        .last()
        .map(|m| m.text.clone())
        .unwrap_or_default();
    assert!(
        last.contains("no mode called") && last.contains("auto, edits, ask, plan or yolo"),
        "{last}"
    );
    assert!(
        p.input.is_empty() && !lines.contains("/approvals"),
        "answered here, never sent as text"
    );
    let _ = std::fs::remove_file(&file);
}

/// `/clear` empties the transcript and the queue, and tells the broker —
/// even while a run is going, which it also stops.
#[test]
fn slash_clear_starts_a_fresh_conversation() {
    let (mut p, file) = pane("clear");
    p.push_note("an old exchange".into());
    p.awaiting = true; // a run is going
    p.queued.push_back("a follow-up".into());
    enter(&mut p, "/clear");
    assert!(
        p.messages.is_empty() && p.queued.is_empty(),
        "transcript and queue are gone"
    );
    let lines = sent(&file, 1);
    assert!(
        lines.contains(r#""text":"/clear""#),
        "the broker is told, past the queue: {lines}"
    );
    let _ = std::fs::remove_file(&file);
}

/// `/approvals default <mode>` switches this pane and asks the app to save the
/// mode for the panes that start later; a saved name that is no mode is dropped
/// on load rather than starting panes nowhere.
#[test]
fn slash_approvals_default_saves_where_new_panes_start() {
    let (mut p, file) = pane("default");
    p.input = "/approvals default plan".into();
    let action = p.on_input(ChatInput::Enter, &std::env::temp_dir());
    assert!(matches!(action, Some(crate::chatkeys::ChatAction::DefaultMode(m)) if m == "plan"));
    assert_eq!(p.approval_mode, ApprovalMode::Plan);
    let cfg = crate::config::CrewConfig {
        approval_default: Some("careful".into()),
        ..Default::default()
    };
    assert_eq!(cfg.clamped().approval_default, None);
    let _ = std::fs::remove_file(&file);
}
