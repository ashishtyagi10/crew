//! Budget-aware assembly of dependency outputs into a worker prompt.
//!
//! Without a budget, [`build_prompt`] concatenates every upstream output
//! whole, so prompt size grows unboundedly with graph fan-in. This module
//! states the budget explicitly and clips at char boundaries with a visible
//! marker, matching the broker's `TASK_CAP`/`clip` idiom in crew-plugin.

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;

use crate::board::TaskResult;

/// Per-dependency cap in chars. Mirrors the broker's `TASK_CAP` (crew-plugin
/// route.rs): ~4k chars ≈ 1k tokens — enough to carry a full upstream answer,
/// small enough that one noisy dep can't crowd out the task's own prompt.
pub(crate) const DEP_CAP: usize = 4_000;

/// Total budget across ALL dependency outputs in one prompt: ~12k chars ≈ 3k
/// tokens, keeping the assembled prompt well inside small-context models even
/// on wide fan-in. When exceeded, the total is split max-min fairly across
/// deps (see [`budgets`]) — heads are kept, no dep is ever dropped whole.
pub(crate) const DEPS_TOTAL_CAP: usize = 12_000;

/// Build a prompt from the task's own prompt plus dependency outputs, each
/// clipped to its char budget. Pure and testable.
pub(crate) fn build_prompt(task_prompt: &str, deps: &[TaskResult]) -> String {
    if deps.is_empty() {
        return task_prompt.to_owned();
    }
    let lens: Vec<usize> = deps.iter().map(|d| d.output.chars().count()).collect();
    let budgets = budgets(&lens);
    let mut out = task_prompt.to_owned();
    out.push_str("\n\nContext from dependencies:\n");
    for (dep, &max) in deps.iter().zip(&budgets) {
        out.push_str("- ");
        out.push_str(&clip_head(&dep.output, max));
        out.push('\n');
    }
    out
}

/// Clip `s` to at most `max` chars, keeping the HEAD and appending a visible
/// truncation marker. Counts chars, never bytes, so multi-byte text is never
/// split mid-char. Under-budget input passes through byte-identical.
fn clip_head(s: &str, max: usize) -> String {
    let total = s.chars().count();
    if total <= max {
        return s.to_owned();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}… [clipped {} chars]", total - max)
}

/// Char budget for each dep: cap each at [`DEP_CAP`]; if the sum still busts
/// [`DEPS_TOTAL_CAP`], split the total max-min fairly — deps already under the
/// fair share keep everything, and the slack they leave is re-shared among the
/// longer ones. Every dep keeps at least its fair share of head chars.
fn budgets(lens: &[usize]) -> Vec<usize> {
    let mut want: Vec<usize> = lens.iter().map(|&l| l.min(DEP_CAP)).collect();
    let mut budget = DEPS_TOTAL_CAP;
    let mut done = vec![false; want.len()];
    loop {
        let open: Vec<usize> = (0..want.len()).filter(|&i| !done[i]).collect();
        if open.is_empty() || open.iter().map(|&i| want[i]).sum::<usize>() <= budget {
            return want;
        }
        let share = budget / open.len();
        let mut settled_any = false;
        for &i in &open {
            if want[i] <= share {
                budget -= want[i];
                done[i] = true;
                settled_any = true;
            }
        }
        if !settled_any {
            // Everyone left wants more than the fair share: give each exactly it.
            for &i in &open {
                want[i] = share;
                done[i] = true;
            }
        }
    }
}
