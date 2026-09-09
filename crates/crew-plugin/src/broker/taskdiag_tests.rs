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

#[test]
fn files_nobody_serves_are_never_asked_about() {
    let mut h = Fake {
        delay: Duration::ZERO,
        asked: vec![],
    };
    let out = lsp_diagnostics(&mut h, &["notes.md".into(), "x.toml".into()], DIAG_BUDGET);
    assert_eq!(out, Diag::NoServer);
    assert!(h.asked.is_empty());
}

#[test]
fn a_clean_answer_counts_the_files_and_a_failure_is_silence() {
    let mut h = Fake {
        delay: Duration::ZERO,
        asked: vec![],
    };
    let files = ["ok1.rs".to_string(), "ok2.rs".into(), "broken.rs".into()];
    assert_eq!(lsp_diagnostics(&mut h, &files, DIAG_BUDGET), Diag::Clean(2));
    // Every server failing is indistinguishable from no server: say nothing
    // rather than claim "no diagnostics" about files nobody looked at.
    assert_eq!(
        lsp_diagnostics(&mut h, &["broken.rs".to_string()], DIAG_BUDGET),
        Diag::NoServer
    );
}

#[test]
fn diagnostics_are_collected_one_line_per_finding() {
    let mut h = Fake {
        delay: Duration::ZERO,
        asked: vec![],
    };
    let files = ["a.rs".to_string(), "ok.rs".into(), "b.rs".into()];
    assert_eq!(
        lsp_diagnostics(&mut h, &files, DIAG_BUDGET),
        Diag::Lines(vec![
            "a.rs:1:1 \u{2014} error [rustc]: boom".into(),
            "b.rs:1:1 \u{2014} error [rustc]: boom".into(),
        ])
    );
}

/// A slow server does not hold the task's ending hostage: the loop stops once
/// the budget is spent and reports what it has.
#[test]
fn the_loop_stops_at_the_budget_and_keeps_what_it_has() {
    let mut h = Fake {
        delay: Duration::from_millis(40),
        asked: vec![],
    };
    let files: Vec<String> = (0..20).map(|i| format!("f{i}.rs")).collect();
    let started = std::time::Instant::now();
    let out = lsp_diagnostics(&mut h, &files, Duration::from_millis(100));
    let took = started.elapsed();
    assert!(
        took < Duration::from_millis(400),
        "ran the whole list: {took:?}"
    );
    let n = h.asked.len();
    assert!((1..20).contains(&n), "asked {n} of 20");
    match out {
        Diag::Lines(lines) => assert_eq!(lines.len(), n, "kept what it had"),
        other => panic!("{other:?}"),
    }
}
