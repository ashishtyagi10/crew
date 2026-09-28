use super::*;
use crate::board::TaskResult;
use crate::graph::TaskId;

fn dep(id: u64, output: String) -> TaskResult {
    TaskResult {
        task: TaskId(id),
        output,
        success: true,
    }
}

/// The marker `clip_middle` leaves where a dep's middle was cut.
const CUT: &str = "chars cut from the middle]";

/// (a) numeric bound, (b) visible cut marker per shortened dep, (c) the HEAD
/// and the TAIL of every dep survive. Four 10k-char deps bust both the per-dep
/// cap (4000) and the total cap (12000), so each dep lands at 12000/4 = 3000.
#[test]
fn oversized_dep_outputs_are_bounded_with_markers_heads_and_tails() {
    let deps: Vec<TaskResult> = (0..4)
        .map(|i| dep(i, format!("H{i}#{}#T{i}", "x".repeat(9_994))))
        .collect();
    let p = build_prompt("do it", &deps);
    // 5 (prompt) + 29 (framing) + 4 × (3000 budget + "- " + 38-char marker
    // line + "\n") comes to 12_198 chars; 12_200 leaves no room for more.
    let n = p.chars().count();
    assert!(n <= 12_200, "prompt must be budget-bounded, got {n} chars");
    assert_eq!(
        p.matches(CUT).count(),
        4,
        "every shortened dep says so with a visible marker"
    );
    for i in 0..4 {
        assert!(
            p.contains(&format!("H{i}#")),
            "head of dep {i} must survive"
        );
        assert!(
            p.contains(&format!("#T{i}\n")),
            "tail of dep {i} must survive"
        );
    }
}

/// (f) A dependency writes its findings first and its verdict last; the task
/// downstream gets the verdict, however long the findings ran.
#[test]
fn a_long_dep_output_hands_its_last_line_downstream() {
    let mut out: String = (1..=200)
        .map(|n| format!("finding {n}: looked at one more thing\n"))
        .collect();
    out.push_str("VERDICT: the fix is in route.rs:412");
    assert!(out.chars().count() > DEP_CAP, "fixture must bust DEP_CAP");
    let p = build_prompt("fix it", &[dep(0, out)]);
    assert!(
        p.contains("VERDICT: the fix is in route.rs:412\n"),
        "the dependency's conclusion was cut: {}",
        &p[p.len() - 200..]
    );
    assert!(p.contains("- finding 1: "), "and its head is kept");
}

/// Max-min fairness: a tiny dep keeps everything; the slack it leaves under an
/// even 12000/5 = 2400 split is redistributed, so each long dep keeps
/// (12000 - 4) / 4 = 2999 chars — not 2400, and never dropped whole.
#[test]
fn short_deps_pass_whole_and_leave_slack_to_long_ones() {
    let fillers = ['q', 'w', 'z', 'j'];
    let mut deps = vec![dep(0, "tiny".into())];
    for (i, f) in fillers.iter().enumerate() {
        deps.push(dep(i as u64 + 1, f.to_string().repeat(10_000)));
    }
    let p = build_prompt("go", &deps);
    assert!(p.contains("- tiny\n"), "under-budget dep is untouched");
    for f in fillers {
        assert_eq!(
            p.matches(f).count(),
            2_999,
            "long dep '{f}' keeps its fair share of chars"
        );
    }
    assert_eq!(p.matches(CUT).count(), 4);
}

/// Multibyte safety: clipping a CJK/emoji dep must cut on a char boundary
/// (no panic, no broken UTF-8) while keeping the head and marking the cut.
#[test]
fn multibyte_output_clips_on_char_boundary_without_panic() {
    let deps = vec![dep(0, "汉字🦀".repeat(5_000))]; // 15_000 chars
    let p = build_prompt("go", &deps);
    assert!(p.contains("- 汉字🦀"), "head survives");
    assert!(p.contains("汉字🦀\n"), "tail survives");
    assert!(p.contains(CUT), "cut is marked");
    let n = p.chars().count();
    assert!(n <= 4_100, "single dep is capped at DEP_CAP, got {n} chars");
    assert!(std::str::from_utf8(p.as_bytes()).is_ok());
}

/// Under-budget outputs pass through byte-identical — no marker, no reflow.
#[test]
fn under_budget_outputs_pass_through_unchanged() {
    let deps = vec![dep(0, "short one".into()), dep(1, "short 二番".into())];
    let p = build_prompt("go", &deps);
    assert!(p.contains("- short one\n"));
    assert!(p.contains("- short 二番\n"));
    assert!(!p.contains(CUT));
    assert!(!p.contains('…'));
}
