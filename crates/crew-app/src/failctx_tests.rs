use super::*;

/// A pane's spans for one command over buffer lines `[from, to)`, finished
/// with `exit` the way `poll` does it: started, closed, then the shell's `D`.
fn spans(from: usize, to: usize, exit: Option<i32>, line: Option<&str>) -> Spans {
    let mut s = Spans::default();
    s.started("cargo".into(), from, 0);
    s.set_line(line.map(Into::into));
    s.close_at(to, 1);
    s.finished(exit, to, 2);
    s
}

/// The failure of the pane's last command, as `failure_at` finds it.
fn failure(spans: &Spans, text: &str, now: usize, cwd: Option<&Path>) -> Option<Failure> {
    failure_of(last_failed(spans)?, text, now, cwd)
}

const SCREEN: &str = "old output\n$ cargo test -p crew-app\nrunning 3 tests\nerror[E0433]: nope\ntest result: FAILED\n$ \nnext prompt\n";

fn failed() -> Failure {
    failure(
        &spans(3, 6, Some(101), Some("cargo test -p crew-app")),
        SCREEN,
        7,
        None,
    )
    .expect("exit 101 is a failure")
}

#[test]
fn a_failed_command_yields_its_own_output_and_nothing_around_it() {
    let f = failed();
    assert_eq!(f.command, "cargo test -p crew-app");
    assert_eq!(f.exit, 101);
    assert_eq!(
        f.output,
        "running 3 tests\nerror[E0433]: nope\ntest result: FAILED"
    );
}

#[test]
fn a_command_that_succeeded_has_no_failure() {
    assert_eq!(failure(&spans(3, 6, Some(0), None), SCREEN, 7, None), None);
}

#[test]
fn a_silent_shell_is_not_a_failure() {
    // No OSC 133: crew cannot tell a failure from a success, so it says none.
    assert_eq!(failure(&spans(3, 6, None, None), SCREEN, 7, None), None);
    assert_eq!(failure(&Spans::default(), SCREEN, 7, None), None);
}

#[test]
fn only_the_last_command_counts() {
    let mut s = spans(3, 6, Some(101), None);
    s.started("ls".into(), 6, 3);
    s.close_at(7, 4);
    s.finished(Some(0), 7, 4);
    assert_eq!(
        failure(&s, SCREEN, 7, None),
        None,
        "a later success clears it"
    );
    s.started("cargo".into(), 7, 5);
    assert_eq!(
        failure(&s, SCREEN, 7, None),
        None,
        "running has no status yet"
    );
}

#[test]
fn the_span_is_found_when_the_capture_starts_after_the_buffer_does() {
    // The capture dropped the first 100 lines the buffer had seen.
    let f = failure(&spans(103, 106, Some(2), None), SCREEN, 107, None).unwrap();
    assert!(f.output.starts_with("running 3 tests"), "{f:?}");
}

#[test]
fn a_long_output_keeps_its_end() {
    let text: String = (0..400)
        .map(|i| format!("line {i}\n"))
        .chain(["$ \n".into()])
        .collect();
    let f = failure(&spans(1, 401, Some(1), None), &text, 401, None).unwrap();
    let lines: Vec<&str> = f.output.lines().collect();
    assert_eq!(lines.len(), KEEP_LINES);
    assert_eq!(lines.last(), Some(&"line 399"));
    assert_eq!(lines.first(), Some(&"line 280"));
}

#[test]
fn a_wide_output_is_capped_in_bytes_and_still_keeps_its_end() {
    let text: String = (0..200)
        .map(|i| format!("{i:>3} {}\n", "x".repeat(96)))
        .chain(["$ \n".into()])
        .collect();
    let f = failure(&spans(1, 201, Some(1), None), &text, 201, None).unwrap();
    assert!(f.output.len() <= KEEP_BYTES, "{}", f.output.len());
    assert!(f.output.ends_with(&"x".repeat(96)));
    assert!(f.output.contains("199 "), "the newest line survives");
    assert!(!f.output.contains("\n  0 "), "the oldest does not");
}

#[test]
fn escapes_are_stripped_from_the_output() {
    let text =
        "\u{1b}[1;31merror\u{1b}[0m: bad \u{1b}]8;;http://x\u{7}link\u{1b}]8;;\u{1b}\\ done\n$ \n";
    let f = failure(&spans(1, 2, Some(1), None), text, 2, None).unwrap();
    assert_eq!(f.output, "error: bad link done");
}

#[test]
fn the_command_is_its_line_or_else_its_name() {
    let s = spans(0, 1, Some(1), None);
    assert_eq!(failure(&s, "x\n", 1, None).unwrap().command, "cargo");
}

#[test]
fn the_folder_is_written_with_a_tilde() {
    let home = dirs::home_dir().expect("a home");
    let f = failure(
        &spans(0, 1, Some(1), None),
        "x\n",
        1,
        Some(&home.join("code").join("crew")),
    )
    .unwrap();
    // The platform's own separator: `~/code/crew`, or `~\code\crew` on Windows.
    let sep = std::path::MAIN_SEPARATOR;
    assert_eq!(f.cwd, Some(format!("~{sep}code{sep}crew")));
}

#[test]
fn the_prefill_names_the_failure_and_fences_its_output() {
    let mut f = failed();
    f.cwd = Some("~/code/crew".into());
    assert_eq!(
        f.prefill(),
        "`cargo test -p crew-app` failed (exit 101) in ~/code/crew. Find the cause and fix it.\n\
         ```\nrunning 3 tests\nerror[E0433]: nope\ntest result: FAILED\n```\n"
    );
}

#[test]
fn the_fence_outgrows_any_backticks_in_the_output() {
    let mut f = failed();
    f.output = "see ```rust``` here".into();
    assert!(f.prefill().contains("\n````\nsee ```rust``` here\n````\n"));
}

#[test]
fn a_failure_with_no_output_is_still_a_question() {
    let mut f = failed();
    f.output.clear();
    assert_eq!(
        f.prefill(),
        "`cargo test -p crew-app` failed (exit 101). Find the cause and fix it."
    );
}

#[test]
fn the_explain_context_leads_with_the_command_and_its_status() {
    assert!(failed()
        .explain_context()
        .starts_with("`cargo test -p crew-app` failed (exit 101)\n\nrunning 3 tests"));
}

#[test]
fn a_secret_in_the_command_reaches_neither_the_prefill_nor_the_context() {
    let mut f = failed();
    f.command = "deploy --token=hunter2 API_KEY=abc".into();
    for text in [f.prefill(), f.explain_context()] {
        assert!(!text.contains("hunter2") && !text.contains("abc"), "{text}");
        assert!(text.contains("deploy --token=*** API_KEY=***"), "{text}");
    }
}
