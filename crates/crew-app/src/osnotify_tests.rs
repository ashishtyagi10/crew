use super::*;
use NotifyKind::*;

fn ask(kind: NotifyKind, focus: Option<bool>, on: bool, detail: &str) -> Option<Banner> {
    decide(kind, focus, on, "build", detail)
}

#[test]
fn focused_or_unknown_window_never_posts() {
    assert_eq!(ask(AgentDone, Some(true), true, "make (42s)"), None);
    assert_eq!(ask(Failed, None, true, "make (42s) \u{2014} exit 2"), None);
}

#[test]
fn setting_off_never_posts() {
    assert_eq!(ask(AgentDone, Some(false), false, "make (42s)"), None);
}

#[test]
fn only_finishes_post() {
    for k in [Bell, Pattern, Exited, Waiting, Requested] {
        assert_eq!(ask(k, Some(false), true, "x"), None, "{k:?}");
    }
}

#[test]
fn a_failed_command_says_exit_and_time() {
    let b = ask(
        Failed,
        Some(false),
        true,
        "cargo test (42s) \u{2014} exit 101",
    )
    .unwrap();
    assert_eq!(b.title, "build");
    assert_eq!(b.body, "cargo test \u{2014} failed (exit 101) \u{b7} 42s");
}

#[test]
fn done_commands_and_agent_turns() {
    let b = ask(AgentDone, Some(false), true, "cargo build (1m12)").unwrap();
    assert_eq!(b.body, "cargo build \u{2014} done \u{b7} 1m12");
    let b = ask(AgentDone, Some(false), true, "turn (1m12)").unwrap();
    assert_eq!(b.body, "agent turn \u{2014} done \u{b7} 1m12");
}

#[test]
fn command_text_is_cut_and_single_line() {
    let long = format!("echo {} (9s)", "x".repeat(200));
    let b = ask(AgentDone, Some(false), true, &long).unwrap();
    assert!(b.body.chars().count() < 80, "{}", b.body);
    assert!(b.body.contains('\u{2026}'));
    let b = ask(AgentDone, Some(false), true, "a\nb\x07c (1s)").unwrap();
    assert!(!b.body.chars().any(|c| c.is_control()));
}

#[test]
fn applescript_escape_quotes_and_backslashes() {
    assert_eq!(applescript_escape(r#"a"b\c"#), r#"a\"b\\c"#);
    let evil = r#"" & (do shell script "rm -rf ~") & ""#;
    let esc = applescript_escape(evil);
    // Every quote is preceded by a backslash: no way to close the string.
    let mut bs = 0;
    for c in esc.chars() {
        if c == '"' {
            assert!(bs % 2 == 1, "unescaped quote in {esc}");
        }
        bs = if c == '\\' { bs + 1 } else { 0 };
    }
}

#[test]
fn script_takes_text_from_argv_not_interpolation() {
    assert!(SCRIPT
        .iter()
        .all(|l| !l.contains("body") && !l.contains('{')));
    assert!(SCRIPT[1].contains("item 1 of argv"));
}

#[test]
fn notify_posts_once_when_away_and_throttles_the_repeat() {
    let mut app = crate::app::CrewApp::default();
    app.win_focus = Some(false);
    sent::take();
    for _ in 0..2 {
        app.notify(
            Failed,
            "api".into(),
            "cargo test (42s) \u{2014} exit 101".into(),
        );
    }
    let got = sent::take();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].title, "api");
    app.win_focus = Some(true);
    app.notify(AgentDone, "api".into(), "make (50s)".into());
    assert!(sent::take().is_empty());
    app.win_focus = Some(false);
    app.config.notify_os = false;
    app.notify(AgentDone, "api".into(), "make (51s)".into());
    assert!(sent::take().is_empty());
}
