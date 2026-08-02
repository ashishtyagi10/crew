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

/// (a) numeric bound, (b) visible clip marker per shortened dep, (c) the HEAD
/// of every dep survives. Four 10k-char deps bust both the per-dep cap (4000)
/// and the total cap (12000), so each dep must land at 12000/4 = 3000 chars.
#[test]
fn oversized_dep_outputs_are_bounded_with_markers_and_heads() {
    let deps: Vec<TaskResult> = (0..4)
        .map(|i| dep(i, format!("H{i}#{}", "x".repeat(9_997))))
        .collect();
    let p = build_prompt("do it", &deps);
    // 5 (prompt) + 29 (framing) + 4 × (3000 budget + "- " + marker + "\n")
    // comes to ~12_134 chars; 12_200 leaves slack only for marker digits.
    let n = p.chars().count();
    assert!(n <= 12_200, "prompt must be budget-bounded, got {n} chars");
    assert_eq!(
        p.matches("[clipped").count(),
        4,
        "every shortened dep says so with a visible marker"
    );
    for i in 0..4 {
        assert!(
            p.contains(&format!("H{i}#")),
            "head of dep {i} must survive"
        );
    }
}

/// Max-min fairness: a tiny dep keeps everything; the slack it leaves under an
/// even 12000/5 = 2400 split is redistributed, so each long dep keeps
/// (12000 - 4) / 4 = 2999 head chars — not 2400, and never dropped whole.
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
            "long dep '{f}' keeps its fair share of head chars"
        );
    }
    assert_eq!(p.matches("[clipped").count(), 4);
}

/// Multibyte safety: clipping a CJK/emoji dep must cut on a char boundary
/// (no panic, no broken UTF-8) while keeping the head and marking the cut.
#[test]
fn multibyte_output_clips_on_char_boundary_without_panic() {
    let deps = vec![dep(0, "汉字🦀".repeat(5_000))]; // 15_000 chars
    let p = build_prompt("go", &deps);
    assert!(p.contains("汉字🦀"), "head survives");
    assert!(p.contains("[clipped"), "cut is marked");
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
    assert!(!p.contains("[clipped"));
    assert!(!p.contains('…'));
}
