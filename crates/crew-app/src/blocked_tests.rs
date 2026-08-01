use super::*;
use crate::app::CrewApp;
use crate::pane::PaneContent;

fn rows(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|s| s.to_string()).collect()
}

fn pane_with(content: PaneContent, born_ms: u64) -> Pane {
    let (x, y, w, h) = (0.0, 0.0, 0.0, 0.0);
    let rect = crate::layout::Rect { x, y, w, h };
    Pane {
        content,
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect,
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms,
    }
}

fn far(born_ms: u64) -> Pane {
    let f = crate::farpane::FarPane::new(std::env::temp_dir());
    pane_with(PaneContent::Far(f), born_ms)
}

fn chat(born_ms: u64, pending: bool) -> Pane {
    let mut c = crate::chat::tests::pane();
    c.plan_pending = pending;
    pane_with(PaneContent::Chat(c), born_ms)
}

#[test]
fn question_prompts_match_in_any_case() {
    assert!(tail_is_prompt(&rows(&["Overwrite existing file? (y/n)"])));
    assert!(tail_is_prompt(&rows(&["Apply changes? [y/N]"])));
    assert!(tail_is_prompt(&rows(&["Continue? (Y/n)"])));
    assert!(tail_is_prompt(&rows(&["Proceed?"])));
    assert!(tail_is_prompt(&rows(&["Press Enter to continue"])));
    // Claude-Code style: question, then a ❯ selector on the tail's last row.
    assert!(tail_is_prompt(&rows(&[
        "Do you want to make this edit?",
        "❯ 1. Yes",
    ])));
    // Permission language near a question mark.
    assert!(tail_is_prompt(&rows(&[
        "claude needs permission to run this tool",
        "Allow this command?",
    ])));
}

#[test]
fn non_prompts_do_not_match() {
    // A bare shell prompt: no question above it, just an idle shell.
    assert!(!tail_is_prompt(&rows(&["~/code/crew", "❯"])));
    assert!(!tail_is_prompt(&rows(&["$"])));
    // An empty grid.
    assert!(!tail_is_prompt(&rows(&[])));
    assert!(!tail_is_prompt(&rows(&["", "", ""])));
    // A question mark alone, with no ❯ / y-n / permission signal.
    assert!(!tail_is_prompt(&rows(&[
        "what is a monad?",
        "a monoid in the category of endofunctors",
    ])));
    // "(y/n)" on screen but above the last TAIL_ROWS non-empty rows —
    // old output scrolled up, with fresh build noise below it.
    assert!(!tail_is_prompt(&rows(&[
        "prompt(\"delete? (y/n)\")",
        "",
        "Compiling a v0.1.0",
        "Compiling b v0.2.0",
        "Compiling c v0.3.0",
        "Compiling d v0.4.0",
        "Building [====>   ]",
    ])));
}

#[test]
fn quiescence_gates_the_prompt_scan() {
    let prompt = rows(&["Proceed? (y/n)"]);
    assert!(!term_waiting(10_000, 8_000, &prompt), "recent = thinking");
    assert!(term_waiting(10_000, 7_000, &prompt), "3s quiet + prompt");
    let busy = rows(&["still working..."]);
    assert!(!term_waiting(10_000, 0, &busy), "quiet but no prompt");
}

#[test]
fn smith_pane_blocked_iff_plan_pending() {
    assert!(pane_blocked(&chat(1, true), 0));
    assert!(!pane_blocked(&chat(1, false), 0));
}

#[test]
fn becomes_blocked_focuses_when_user_idle() {
    let mut st = BlockedState::default();
    let up = st.update(&[(1, false), (2, true)], Some(1), false, true);
    assert_eq!(up.newly, vec![2]);
    assert_eq!(up.focus, Some(2));
}

#[test]
fn no_steal_while_typing_or_onto_a_blocked_focus() {
    // The user typed recently: the badge still raises, focus stays put.
    let mut st = BlockedState::default();
    let up = st.update(&[(1, false), (2, true)], Some(1), false, false);
    assert_eq!((up.newly, up.focus), (vec![2], None));
    // The focused pane is itself blocked: same outcome.
    let mut st = BlockedState::default();
    let up = st.update(&[(1, true), (2, true)], Some(1), true, true);
    assert_eq!((up.newly, up.focus), (vec![2], None));
}

#[test]
fn focused_pane_is_never_newly_blocked() {
    let mut st = BlockedState::default();
    let up = st.update(&[(1, true)], Some(1), true, true);
    assert!(up.newly.is_empty(), "the user is already looking at it");
    assert_eq!(up.focus, None);
}

#[test]
fn due_throttles_to_one_check_per_second() {
    let mut st = BlockedState::default();
    assert!(st.due(0));
    assert!(!st.due(CHECK_EVERY_MS - 1));
    assert!(st.due(CHECK_EVERY_MS));
}

#[test]
fn one_auto_focus_per_episode() {
    let mut st = BlockedState::default();
    assert_eq!(st.update(&[(2, true)], None, false, true).focus, Some(2));
    let again = st.update(&[(2, true)], None, false, true).focus;
    assert_eq!(again, None, "same episode never refocuses");
    st.update(&[(2, false)], None, false, true); // the prompt was answered
    let fresh = st.update(&[(2, true)], None, false, true).focus;
    assert_eq!(fresh, Some(2), "a NEW episode surfaces again");
}

#[test]
fn tick_blocked_autofocuses_and_badges_the_waiting_pane() {
    let mut app = CrewApp::default();
    app.panes.push(far(1));
    app.panes.push(chat(2, true));
    app.last_input_ms = 0; // hands-off since forever
    assert!(app.tick_blocked(100_000), "something changed → repaint");
    assert_eq!(app.focused, 1, "focus moved to the waiting pane");
    assert!(app.panes[1].attention.is_some(), "waiting badge raised");
}

#[test]
fn tick_blocked_never_steals_while_typing() {
    let mut app = CrewApp::default();
    app.panes.push(far(1));
    app.panes.push(chat(2, true));
    app.last_input_ms = 99_000; // typed 1s ago
    app.tick_blocked(100_000);
    assert_eq!(app.focused, 0, "recent typing blocks the steal");
    let badge = app.panes[1].attention;
    assert!(badge.is_some(), "…but the badge still shows");
}

#[test]
fn tick_blocked_does_not_ping_pong() {
    let mut app = CrewApp::default();
    app.panes.push(far(1));
    app.panes.push(chat(2, true));
    app.last_input_ms = 0;
    app.tick_blocked(100_000);
    assert_eq!(app.focused, 1);
    app.focused = 0; // the user deliberately goes back
    app.tick_blocked(102_000);
    assert_eq!(app.focused, 0, "one auto-focus per blocked episode");
}

#[test]
fn cmd_period_cycles_blocked_panes_and_noops_empty() {
    let mut app = CrewApp::default();
    app.panes.push(far(1));
    app.panes.push(chat(2, true));
    app.panes.push(chat(3, true));
    app.focus_next_blocked();
    assert_eq!(app.focused, 1);
    app.focus_next_blocked();
    assert_eq!(app.focused, 2);
    app.focus_next_blocked();
    assert_eq!(app.focused, 1, "wraps past the end");
    // With nothing waiting: a no-op plus a status note.
    let mut app = CrewApp::default();
    app.panes.push(far(1));
    app.focus_next_blocked();
    assert_eq!(app.focused, 0);
    let (msg, _) = app.status.clone().expect("a status note");
    assert!(msg.contains("no pane is waiting"), "got: {msg}");
}
