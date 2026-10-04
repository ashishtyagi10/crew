use std::time::{Duration, Instant};

use crew_plugin::{ApprovalMode, Plugin};
use winit::keyboard::{Key, NamedKey};

use crate::chat::ChatPane;
use crate::chatkeys::{chat_key, ChatInput};

/// A pane whose "broker" writes every command it is sent to a file, so a
/// test can read back what the pane told it.
fn pane() -> (ChatPane, std::path::PathBuf) {
    let file = std::env::temp_dir().join(format!(
        "crew-chatapprove-{}-{:?}.jsonl",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_file(&file);
    let sh = format!("cat > '{}'", file.display());
    let plugin = Plugin::spawn("sh", &["-c".to_string(), sh]).unwrap();
    (ChatPane::new(plugin, "crew".into()), file)
}

/// The commands the broker has received so far, once `want` of them landed.
fn sent(file: &std::path::Path, want: usize) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        if text.lines().count() >= want || Instant::now() > deadline {
            return text.lines().map(str::to_string).collect();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn footer(p: &ChatPane) -> String {
    let fc = crate::chatsummary::footer_ctx(p);
    crate::summaryroute::route_line(&fc, 100)
        .iter()
        .map(|c| c.0)
        .collect()
}

/// Shift+Tab is its own key — not Tab's completion — and each press steps
/// the mode, tells the broker and says what the mode means; four presses
/// come back round to auto.
#[test]
fn shift_tab_steps_the_mode_and_tells_the_broker() {
    let tab = Key::Named(NamedKey::Tab);
    assert!(matches!(
        chat_key(&tab, true, true, false),
        ChatInput::CycleMode
    ));
    assert!(matches!(
        chat_key(&tab, true, false, false),
        ChatInput::Complete
    ));
    let (mut p, file) = pane();
    let cwd = std::env::temp_dir();
    let mut seen = Vec::new();
    for _ in 0..4 {
        assert!(p.on_input(ChatInput::CycleMode, &cwd).is_none());
        seen.push(p.approval_mode);
    }
    use ApprovalMode::*;
    assert_eq!(seen, [Edits, Ask, Plan, Auto]);
    let lines = sent(&file, 4);
    for (line, mode) in lines.iter().zip(["edits", "ask", "plan", "auto"]) {
        assert!(
            line.contains(r#""type":"mode""#) && line.contains(&format!(r#""approval":"{mode}""#)),
            "{line}"
        );
    }
    let notes: Vec<&str> = p.messages.iter().map(|m| m.text.as_str()).collect();
    assert!(
        notes.iter().any(|n| n.starts_with("plan only")),
        "{notes:?}"
    );
    let _ = std::fs::remove_file(&file);
}

/// The mode wears a badge on the footer, except the default; idle, the
/// footer says which key changes it.
#[test]
fn the_footer_wears_the_mode_and_says_how_to_change_it() {
    let (mut p, _) = pane();
    assert!(!footer(&p).contains("auto-approve"), "{}", footer(&p));
    assert!(footer(&p).contains("shift+tab"), "{}", footer(&p));
    p.cycle_mode();
    p.cycle_mode();
    p.cycle_mode();
    assert!(footer(&p).contains("plan only"), "{}", footer(&p));
}

/// A tool call waiting on a yes: the question shows in the transcript and on
/// the footer, and Enter on an empty composer allows it, by its id.
#[test]
fn enter_allows_the_waiting_tool_call() {
    let (mut p, file) = pane();
    let cwd = std::env::temp_dir();
    p.ask_user(
        "a7".into(),
        "run `cargo test` \u{2014} it cannot be undone".into(),
    );
    assert!(
        footer(&p).contains("allow? run `cargo test`"),
        "{}",
        footer(&p)
    );
    assert!(footer(&p).contains("enter allows"), "{}", footer(&p));
    assert!(p
        .messages
        .last()
        .is_some_and(|m| m.text.contains("run `cargo test`")));
    assert!(p.on_input(ChatInput::Enter, &cwd).is_none());
    assert!(p.asking.is_empty(), "answered");
    let lines = sent(&file, 1);
    assert!(
        lines.iter().any(|l| l.contains(r#""type":"approve""#)
            && l.contains(r#""id":"a7""#)
            && l.contains(r#""granted":true"#)),
        "{lines:?}"
    );
    let _ = std::fs::remove_file(&file);
}

/// Esc refuses it, and so does a typed `no`; anything else typed is a
/// message, and leaves the question waiting.
#[test]
fn esc_or_no_refuses_and_other_text_is_a_message() {
    let cwd = std::env::temp_dir();
    for refuse in ["esc", "no"] {
        let (mut p, file) = pane();
        p.ask_user("a1".into(), "edit src/main.rs".into());
        match refuse {
            "esc" => assert!(
                p.on_input(ChatInput::Close, &cwd).is_none(),
                "esc must not close"
            ),
            _ => {
                p.input = "no".into();
                p.on_input(ChatInput::Enter, &cwd);
            }
        }
        assert!(p.asking.is_empty(), "{refuse} answered it");
        let lines = sent(&file, 1);
        assert!(
            lines.iter().any(|l| l.contains(r#""granted":false"#)),
            "{refuse}: {lines:?}"
        );
        let _ = std::fs::remove_file(&file);
    }
    let (mut p, _) = pane();
    p.ask_user("a2".into(), "edit src/main.rs".into());
    p.input = "why do you need that?".into();
    assert!(
        p.approval_key(&ChatInput::Enter).is_none(),
        "a message, not an answer"
    );
    assert!(!p.asking.is_empty(), "the question still waits");
}

/// When the task ends — the broker gave up waiting, or it was stopped —
/// there is nothing left to answer, and the footer stops asking.
#[test]
fn a_finished_task_takes_its_question_with_it() {
    let (mut p, _) = pane();
    p.absorb_task(3, true);
    p.ask_user("a3".into(), "run `make`".into());
    p.absorb_task(3, false);
    assert!(p.asking.is_empty());
    assert!(!footer(&p).contains("allow?"), "{}", footer(&p));
}

/// Two agents of one swarm asking at once: both questions queue, the footer
/// says one waits behind the other, and each answer goes to its own id.
#[test]
fn questions_asked_together_are_answered_in_turn() {
    let (mut p, file) = pane();
    let cwd = std::env::temp_dir();
    p.ask_user("a1".into(), "edit a.rs".into());
    p.ask_user("a2".into(), "edit b.rs".into());
    assert!(
        footer(&p).contains("allow? edit a.rs (+1 more)"),
        "{}",
        footer(&p)
    );
    p.on_input(ChatInput::Enter, &cwd);
    assert!(footer(&p).contains("allow? edit b.rs"), "{}", footer(&p));
    p.on_input(ChatInput::Close, &cwd);
    assert!(p.asking.is_empty());
    let lines = sent(&file, 2);
    let a1 = lines
        .iter()
        .any(|l| l.contains(r#""id":"a1""#) && l.contains(r#""granted":true"#));
    let a2 = lines
        .iter()
        .any(|l| l.contains(r#""id":"a2""#) && l.contains(r#""granted":false"#));
    assert!(a1 && a2, "{lines:?}");
    let _ = std::fs::remove_file(&file);
}
