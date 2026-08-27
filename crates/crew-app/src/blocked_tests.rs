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
        glide: crate::glide::Glide::default(),
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

/// Claude Code 2.1.222's Bash-approval screen, captured 2026-08-05 from a real
/// `claude` run in a 44-column PTY (a typical narrow crew grid pane): option 2
/// and the hint line wrap, pushing the question 8 non-empty rows above the
/// bottom.
const CLAUDE_44COL_APPROVAL: &[&str] = &[
    "⏺ Running 1 shell command…",
    "  ⎿  $ rm -f /tmp/nonexistent_xyz_12345",
    "",
    "────────────────────────────────────────────",
    " Bash command",
    "",
    "   rm -f /tmp/nonexistent_xyz_12345",
    "   Remove non-existent file with force",
    "   flag",
    "",
    " Do you want to proceed?",
    " ❯ 1. Yes",
    "   2. Yes, and always allow access to tmp/",
    "      from this project",
    "   3. No",
    "",
    " Esc to cancel · Tab to amend · ctrl+e to",
    " explain",
];

/// Codex's exec-approval modal, verbatim from openai/codex's own TUI snapshot
/// test (`approval_modal_exec.snap`): `›` selector, not `❯`.
const CODEX_EXEC_APPROVAL: &[&str] = &[
    "  Would you like to run the following command?",
    "",
    "  Reason: this is a test reason such as one that would be produced by the model",
    "",
    "  $ echo hello world",
    "",
    "› 1. Yes, proceed (y)",
    "  2. Yes, and don't ask again for commands that start with `echo hello world` (p)",
    "  3. No, and tell Codex what to do differently (esc)",
    "",
    "  Press enter to confirm or esc to cancel",
];

#[test]
fn claude_code_narrow_pane_approval_matches() {
    assert!(tail_is_prompt(&rows(CLAUDE_44COL_APPROVAL)));
}

#[test]
fn codex_approval_modal_matches() {
    assert!(tail_is_prompt(&rows(CODEX_EXEC_APPROVAL)));
}

#[test]
fn selector_under_question_matches_without_stock_phrases() {
    // A nonstandard question with no PROMPT_PATTERNS / permission words: the
    // `›`/`❯` option selector under a `?` row is the approval-menu signal.
    assert!(tail_is_prompt(&rows(&[
        "Run `git push` now?",
        "› 1. Yes",
        "  2. No, and tell Codex what to do differently (esc)",
    ])));
}

#[test]
fn blink_does_not_reset_stability() {
    // Claude Code blinks the ⏺ glyph (⏺ ↔ space) every ~600 ms while the
    // approval dialog waits (measured 2026-08-05: 47–49 bytes each ~0.6 s).
    // The swap happens >STABLE_ROWS above the bottom, so the tail hash must
    // hold still and 3 s of wall clock must open the gate.
    let mut w = TailWatch::default();
    let a = rows(CLAUDE_44COL_APPROVAL);
    let mut blinked: Vec<&str> = CLAUDE_44COL_APPROVAL.to_vec();
    blinked[0] = "  Running 1 shell command…"; // ⏺ blinked off
    let b = rows(&blinked);
    w.step(&a, 1_000);
    assert!(!w.waiting(1_000), "just appeared: not yet stable");
    w.step(&b, 2_000);
    w.step(&a, 3_000);
    assert!(
        w.waiting(4_000),
        "blink above the dialog must not reset the clock"
    );
}

#[test]
fn ticking_spinner_is_never_stable() {
    // Thinking state: Claude Code's spinner line rewrites its elapsed-seconds
    // counter every second. Even with question-looking text above it, the
    // tail never goes stable, so the pane never reads as blocked.
    let mut w = TailWatch::default();
    for s in 0..10u64 {
        let mut r = rows(&["Do you want to proceed?", "❯ 1. Yes"]);
        r.push(format!("✳ Pondering… ({s}s · esc to interrupt)"));
        w.step(&r, 1_000 * s);
    }
    assert!(!w.waiting(9_000), "a moving tail is thinking, not waiting");
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
    // "(y/n)" on screen but above the last MATCH_ROWS non-empty rows —
    // old output scrolled up, with fresh build noise below it.
    let mut scrolled = vec!["prompt(\"delete? (y/n)\")".to_string(), String::new()];
    scrolled.extend((0..MATCH_ROWS).map(|i| format!("Compiling crate{i} v0.{i}.0")));
    scrolled.push("Building [====>   ]".to_string());
    assert!(!tail_is_prompt(&scrolled));
}

#[test]
fn stability_gates_the_prompt_scan() {
    let mut w = TailWatch::default();
    w.step(&rows(&["Proceed? (y/n)"]), 7_000);
    assert!(!w.waiting(9_999), "not yet 3s stable = thinking");
    assert!(w.waiting(10_000), "3s stable + prompt");
    w.step(&rows(&["still working..."]), 11_000);
    assert!(!w.waiting(20_000), "stable but no prompt");
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
fn focus_deferred_until_user_goes_idle() {
    // The pane blocks WHILE the user is typing: badge now, no steal — but the
    // focus must still happen once the user goes hands-off, not be forfeited
    // because the rising edge landed inside the busy window.
    let mut st = BlockedState::default();
    let up = st.update(&[(2, true)], None, false, false);
    assert_eq!((up.newly, up.focus), (vec![2], None));
    let up = st.update(&[(2, true)], None, false, true);
    assert!(up.newly.is_empty(), "no second badge");
    assert_eq!(up.focus, Some(2), "focus lands once the user is idle");
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

/// End-to-end against a REAL PTY: a script that behaves like Claude Code 2.1
/// (brief quiet work, then the captured 44-col approval dialog, then a ⏺
/// blink every 300 ms — the exact traffic that defeated the old byte-quiet
/// gate) must auto-focus its pane, and only after the stability window.
// Drives a real PTY running a POSIX shell: Unix-only by construction.
// Windows has no `sh`, so the spawn fails on a detail that says nothing
// about the behaviour under test.
#[cfg(unix)]
#[test]
fn live_pty_claude_style_dialog_autofocuses() {
    use crate::layout::Rect;
    use crate::pane::{Pane, TermPane};
    use crew_term::{GridSize, PtyTerm};
    use std::io::Write;
    use std::time::{Duration, Instant};

    // The dialog text lives in a script file so the command line sent to the
    // shell stays short — a long echoed command would put the dialog strings
    // on screen before the "agent" prints them.
    let dialog: String = CLAUDE_44COL_APPROVAL
        .iter()
        .map(|l| format!("printf '%s\\n' \"{}\"\n", l.replace('$', "\\$")))
        .collect();
    let script = format!(
        "printf 'analyzing repo...\\n'\nsleep 1\n{dialog}\
         while :; do\n\
           sleep 0.15; printf '\\0337\\033[18A\\r \\0338'\n\
           sleep 0.15; printf '\\0337\\033[18A\\r\u{23FA}\\0338'\n\
         done\n"
    );
    let path = std::env::temp_dir().join("crew_blocked_replay.sh");
    std::fs::write(&path, script).unwrap();

    let grid = GridSize { cols: 60, rows: 24 };
    let pty = PtyTerm::spawn(grid, "sh").unwrap();
    {
        let mut w = pty.writer();
        writeln!(w, "sh {}", path.display()).unwrap();
        w.flush().unwrap();
    }
    let input = pty.writer();
    let mut app = CrewApp::default();
    app.panes.push(far(1));
    app.panes.push(Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input,
            // The gate needs a running foreground command; naming it is
            // procnames' job (tested elsewhere), pinned open here.
            cmd: Some("sh".into()),
            cmd_since: None,
            tail: Default::default(),
            read_at: 0,
            spans: Default::default(),
        })),
        grid,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 2,
    });
    app.last_input_ms = 0; // hands-off since forever

    let start = Instant::now();
    let deadline = start + Duration::from_secs(15);
    let mut last_bytes: Option<Instant> = None;
    let mut focused_at = None;
    while Instant::now() < deadline {
        if let PaneContent::Terminal(t) = &mut app.panes[1].content {
            if t.pty.try_read() > 0 {
                last_bytes = Some(Instant::now());
            }
        }
        app.tick_blocked(crate::anim::now_ms());
        if app.focused == 1 {
            focused_at = Some(Instant::now());
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let focused_at = focused_at.expect("the dialog pane was never auto-focused");
    assert!(
        focused_at - start >= Duration::from_secs(3),
        "focus before the 3s stability window: gate did not gate"
    );
    assert!(
        last_bytes.expect("no PTY output at all").elapsed() < Duration::from_secs(1),
        "the ⏺ blink should have kept bytes flowing right up to detection \
         (this is the traffic byte-quiescence could never see past)"
    );
    assert!(app.panes[1].attention.is_some(), "waiting badge raised");
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
