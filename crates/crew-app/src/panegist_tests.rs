//! The gist of real agent screens: what each one is doing, not its footer.
use super::*;

fn rows(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|s| s.to_string()).collect()
}

/// Claude Code thinking: its timer line, under the input box and over the
/// footer — the footer is what the pane's last line said.
#[test]
fn claude_working_says_its_spinner_line_and_time() {
    let screen = rows(&[
        "⏺ Reading src/gen.rs",
        "",
        "✳ Pondering… (12s · ↑ 1.2k tokens · esc to interrupt)",
        "",
        "╭──────────────────────────────╮",
        "│ >                            │",
        "╰──────────────────────────────╯",
        "  ? for shortcuts",
    ]);
    assert_eq!(summary(&screen).as_deref(), Some("Pondering… · 12s"));
}

#[test]
fn codex_and_gemini_working_lines_read_the_same_way() {
    let codex = rows(&["• Working (3m20s • esc to interrupt)", "› ", "  ⏎ send"]);
    assert_eq!(summary(&codex).as_deref(), Some("Working · 3m20s"));
    let gemini = rows(&["⠏ Thinking... (esc to cancel, 45s)", "> "]);
    assert_eq!(summary(&gemini).as_deref(), Some("Thinking... · 45s"));
}

/// Waiting on you: the question its dialog asks, out of its box.
#[test]
fn a_waiting_agent_says_its_question() {
    let screen = rows(&[
        "⏺ Bash(cargo test)",
        "╭──────────────────────────────╮",
        "│ Bash command                 │",
        "│   cargo test                 │",
        "│ Do you want to proceed?      │",
        "│ ❯ 1. Yes                     │",
        "│   2. No                      │",
        "╰──────────────────────────────╯",
    ]);
    assert_eq!(summary(&screen).as_deref(), Some("Do you want to proceed?"));
}

/// Idle: its last message, marker dropped — not the empty prompt under it.
#[test]
fn an_idle_agent_says_its_last_message() {
    let screen = rows(&[
        "⏺ Updated 3 files and ran the tests: all green.",
        "",
        "╭──────────────────────────────╮",
        "│ >                            │",
        "╰──────────────────────────────╯",
        "  ⏵⏵ accept edits on (shift+tab to cycle)",
    ]);
    let got = summary(&screen);
    assert_eq!(got.as_deref(), Some("Updated 3 files and ran the tests: all green."));
}

/// Any other program: its last row that is not chrome.
#[test]
fn a_build_says_its_last_line() {
    let screen = rows(&["   Compiling crew-app v0.30.24", "    Building [====>   ] 512/800"]);
    assert_eq!(summary(&screen).as_deref(), Some("Building [====> ] 512/800"));
    assert_eq!(summary(&rows(&["", "  "])), None);
}

#[test]
fn moving_holds_for_two_steps_then_lapses() {
    let mut g = Gist::default();
    assert!(!g.moving(0), "never observed");
    assert!(g.note(&rows(&["a"]), 1_000), "first sight is news");
    assert!(g.moving(2_000));
    assert!(!g.note(&rows(&["a"]), 2_000), "same words, still moving");
    assert!(g.moving(3_000), "still within the window");
    assert!(!g.moving(1_000 + MOVING_MS), "a still tail stops working");
    assert!(g.note(&rows(&["b"]), 4_000));
    assert!(g.moving(4_500));
    assert!(g.clear());
    assert!(!g.moving(4_500));
}

/// Working lapsing to still is news even though the words did not change:
/// the entry's state word and spine must follow it.
#[test]
fn a_lapse_into_stillness_is_news() {
    let mut g = Gist::default();
    g.note(&rows(&["a"]), 1_000);
    assert!(g.note(&rows(&["a"]), 1_000 + MOVING_MS), "stopped working");
    assert!(!g.note(&rows(&["a"]), 2_000 + MOVING_MS), "and stays stopped");
}
