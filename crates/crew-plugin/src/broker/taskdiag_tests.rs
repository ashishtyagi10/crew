use std::time::Duration;

use super::*;
use crate::broker::taskdiff::DIAG_BUDGET;

// ---- the budget loop ------------------------------------------------------

/// A host that serves `.rs` only, takes `delay` per file, and answers with
/// one diagnostic per file (or a clean line for names containing "ok").
struct Fake {
    delay: Duration,
    asked: Vec<String>,
}

impl DiagSource for Fake {
    fn serves(&self, path: &str) -> bool {
        path.ends_with(".rs")
    }
    fn diagnostics(&mut self, path: &str) -> Result<String, String> {
        std::thread::sleep(self.delay);
        self.asked.push(path.to_string());
        if path.contains("broken") {
            Err("server died".into())
        } else if path.contains("ok") {
            Ok(format!("no diagnostics for {path}"))
        } else {
            Ok(format!("{path}:1:1 \u{2014} error [rustc]: boom"))
        }
    }
}

fn fake(delay: Duration) -> Arc<Mutex<Fake>> {
    Arc::new(Mutex::new(Fake {
        delay,
        asked: vec![],
    }))
}

fn asked(h: &Arc<Mutex<Fake>>) -> Vec<String> {
    h.lock().unwrap().asked.clone()
}

#[test]
fn files_nobody_serves_are_never_asked_about() {
    let h = fake(Duration::ZERO);
    let out = lsp_diagnostics(&h, &["notes.md".into(), "x.toml".into()], DIAG_BUDGET);
    assert_eq!(out, Diag::NoServer);
    assert!(asked(&h).is_empty());
}

#[test]
fn a_clean_answer_counts_the_files_and_a_failure_is_silence() {
    let h = fake(Duration::ZERO);
    let files = ["ok1.rs".to_string(), "ok2.rs".into(), "broken.rs".into()];
    assert_eq!(lsp_diagnostics(&h, &files, DIAG_BUDGET), Diag::Clean(2));
    // Every server failing is indistinguishable from no server: say nothing
    // rather than claim "no diagnostics" about files nobody looked at.
    assert_eq!(
        lsp_diagnostics(&h, &["broken.rs".to_string()], DIAG_BUDGET),
        Diag::NoServer
    );
}

#[test]
fn diagnostics_are_collected_one_line_per_finding() {
    let h = fake(Duration::ZERO);
    let files = ["a.rs".to_string(), "ok.rs".into(), "b.rs".into()];
    assert_eq!(
        lsp_diagnostics(&h, &files, DIAG_BUDGET),
        Diag::Lines(vec![
            "a.rs:1:1 \u{2014} error [rustc]: boom".into(),
            "b.rs:1:1 \u{2014} error [rustc]: boom".into(),
        ])
    );
}

/// A slow server does not hold the task's ending hostage: the loop stops once
/// the budget is spent and reports what came back before it.
#[test]
fn the_loop_stops_at_the_budget_and_keeps_what_it_has() {
    let h = fake(Duration::from_millis(40));
    let files: Vec<String> = (0..20).map(|i| format!("f{i}.rs")).collect();
    let started = Instant::now();
    let out = lsp_diagnostics(&h, &files, Duration::from_millis(100));
    let took = started.elapsed();
    assert!(
        took < Duration::from_millis(400),
        "ran the whole list: {took:?}"
    );
    let lines = match out {
        Diag::Lines(lines) => lines,
        other => panic!("{other:?}"),
    };
    // The ask in flight at the deadline may still finish on its thread; it is
    // not in the report, and nothing is asked after it.
    std::thread::sleep(Duration::from_millis(150));
    let n = asked(&h).len();
    assert!((1..20).contains(&lines.len()), "kept {} of 20", lines.len());
    assert!((lines.len()..=lines.len() + 1).contains(&n), "asked {n}");
}

/// The budget is a deadline, not a check before each ask. A server still
/// indexing after a cold start held the first ask for as long as it took —
/// the diff and its verdict landed at +54.9 s on a task whose summary was out
/// at +42.5 s — and a server that is not ready in time is silence.
#[test]
fn one_ask_slower_than_the_budget_is_cut_off_at_the_budget() {
    let h = fake(Duration::from_secs(2));
    let started = Instant::now();
    let out = lsp_diagnostics(&h, &["cold.rs".to_string()], Duration::from_millis(300));
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(1),
        "waited out the server: {took:?}"
    );
    assert_eq!(out, Diag::NoServer);
}

// ---- the order the task's ending speaks in --------------------------------

/// A host that writes the moment it is first asked into the same log the
/// pane's messages go to.
struct Logged(Arc<Mutex<Vec<String>>>);

impl DiagSource for Logged {
    fn serves(&self, _: &str) -> bool {
        true
    }
    fn diagnostics(&mut self, path: &str) -> Result<String, String> {
        self.0.lock().unwrap().push(format!("asked {path}"));
        Ok(format!("no diagnostics for {path}"))
    }
}

/// The diff is on screen before the language server is asked anything, so
/// however long the server takes, the diff does not wait for it.
#[test]
fn the_diff_is_emitted_before_the_server_is_asked() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let host = Arc::new(Mutex::new(Logged(Arc::clone(&log))));
    let files = ["a.rs".to_string()];
    crate::broker::taskdiff::deliver(
        "+fn a() {}",
        &files,
        |f| lsp_diagnostics(&host, f, DIAG_BUDGET),
        &mut |m| log.lock().unwrap().push(m),
    );
    let log = log.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![
            "```diff\n+fn a() {}\n```".to_string(),
            "asked a.rs".into(),
            "no diagnostics in the changed file".into(),
        ]
    );
}
