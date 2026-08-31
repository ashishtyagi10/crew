use super::*;

#[test]
fn explain_parses_before_ask() {
    assert_eq!(
        explain_command("??why did this fail"),
        Some("why did this fail")
    );
    assert_eq!(explain_command("?? "), Some(""));
    assert_eq!(explain_command("?one mark"), None);
    assert_eq!(explain_command("ls"), None);
    // a `??` line must never read as a `?` ask for "?why…"
    assert_eq!(qmark_command("??why"), Some("?why"));
}

#[test]
fn explain_result_opens_the_markdown_viewer() {
    let mut app = CrewApp::default();
    app.absorb_explain_result(Ok("## It failed\nBecause of X.".into()));
    let last = app.panes.last().expect("a viewer pane opened");
    assert!(
        matches!(last.content, crate::pane::PaneContent::View(_)),
        "the answer opens in the file viewer"
    );
    assert!(app.zoomed, "the viewer opens zoomed");
}

#[test]
fn explain_result_marks_its_viewer_ephemeral() {
    // Fix 4: `??` opens its viewer on a SYNTHETIC temp file (the answer,
    // written to `$TMPDIR`), not something the user asked to view —
    // saving it like a normal viewer would let a run whose only pane is
    // an explanation silently replace a saved session on quit.
    let mut app = CrewApp::default();
    app.absorb_explain_result(Ok("## It failed\nBecause of X.".into()));
    let crate::pane::PaneContent::View(v) = &app.panes.last().unwrap().content else {
        panic!("expected a View pane");
    };
    assert!(
        v.ephemeral,
        "a viewer opened on a synthetic temp file must be marked ephemeral"
    );
}

#[test]
fn explain_errors_reach_the_status_line() {
    let mut app = CrewApp::default();
    app.absorb_explain_result(Err("no AI provider".into()));
    assert!(app
        .active_status()
        .unwrap_or_default()
        .contains("no AI provider"));
    assert!(app.panes.is_empty(), "no pane on error");
}

#[test]
fn context_tail_bounds_lines_and_bytes() {
    let many: String = (0..500).map(|i| format!("line {i}\n")).collect();
    let tail = context_tail(&many, 120, 8 * 1024);
    assert!(tail.lines().count() <= 120);
    assert!(
        tail.ends_with("line 499"),
        "keeps the newest lines: …{tail:.20}"
    );
    let fat = "x".repeat(100_000);
    assert!(context_tail(&fat, 120, 8 * 1024).len() <= 8 * 1024);
}

#[test]
fn qmark_parses_the_query() {
    assert_eq!(qmark_command("?list files"), Some("list files"));
    assert_eq!(qmark_command("?  kill port 8080 "), Some("kill port 8080"));
    assert_eq!(qmark_command("?"), Some(""));
    assert_eq!(qmark_command("ls?"), None);
    assert_eq!(qmark_command("what?"), None);
}

#[test]
fn suggestion_fills_an_empty_bar_ready_to_run() {
    let mut app = CrewApp::default();
    app.absorb_ask_result(Ok("ls -la".into()));
    assert_eq!(app.input.text, "ls -la");
    assert!(app.input.focused, "the bar is focused for the Enter");
    let s = app.active_status().unwrap_or_default();
    assert!(s.contains("Enter"), "status invites the run: {s}");
}

#[test]
fn suggestion_never_clobbers_text_typed_meanwhile() {
    let mut app = CrewApp::default();
    app.input.text = "git st".into();
    app.absorb_ask_result(Ok("ls -la".into()));
    assert_eq!(app.input.text, "git st");
    let s = app.active_status().unwrap_or_default();
    assert!(s.contains("ls -la"), "the suggestion still surfaces: {s}");
}

#[test]
fn empty_suggestion_and_errors_reach_the_status_line() {
    let mut app = CrewApp::default();
    app.absorb_ask_result(Ok("  ".into()));
    assert!(app
        .active_status()
        .unwrap_or_default()
        .contains("no command"));
    app.absorb_ask_result(Err("no AI provider — set DASHSCOPE_API_KEY".into()));
    assert!(app
        .active_status()
        .unwrap_or_default()
        .contains("no AI provider"));
}
