//! Prompt assembly for [`super::ApiAgent`]'s tool rounds.
//!
//! Split out of `apiagent/mod.rs` so the agent's `run` stays readable: this
//! module is pure string work and is tested as such, with no provider, bus or
//! runtime in sight.

#[cfg(test)]
#[path = "toolloop_tests.rs"]
mod tests;

/// Cap on one tool result fed back into the next prompt, in chars.
///
/// Mirrors the relay's 6k. A tool that returns a 200 KB JSON page — an API
/// listing, a directory, a file read — would otherwise blow the next request
/// past the model's context and fail the task at the exact moment the tool
/// SUCCEEDED, which is the most confusing failure this loop can produce.
pub(super) const RESULT_CAP: usize = 6_000;

/// Clip to `max` chars keeping the head, with a visible marker. Counts chars,
/// never bytes, so a multi-byte result is never split mid-character.
pub(super) fn clip(s: &str, max: usize) -> String {
    let total = s.chars().count();
    if total <= max {
        return s.to_owned();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}… [clipped {} chars]", total - max)
}

/// One CALLED/RESULT pair, in the form the next prompt shows the agent.
pub(super) fn exchange(label: &str, args: &str, result: &str) -> String {
    format!(
        "CALLED {label} {args}\nRESULT:\n{}",
        clip(result, RESULT_CAP)
    )
}

/// The prompt for the round after a tool ran: the original task (tools hint
/// included, so a second call is still spellable) plus every exchange so far.
///
/// `rounds_left` is stated to the agent rather than merely enforced. A budget
/// an agent cannot see is one it plans straight past, and then the task ends
/// mid-sequence with a tool call nobody ran — the relay learned this the
/// expensive way and the wording is carried over deliberately.
pub(super) fn follow_up(base: &str, exchanges: &[String], rounds_left: u32) -> String {
    let budget = if rounds_left == 0 {
        "This was your LAST tool call for this task: answer with what you have now.".to_string()
    } else {
        format!("You may make {rounds_left} more tool call(s) for this task.")
    };
    format!(
        "{base}\n\nTOOL EXCHANGES SO FAR:\n{}\n\n{budget} Continue the task using these \
         results. You may call another tool, or give your final answer.",
        exchanges.join("\n\n")
    )
}

/// What an agent's output becomes when it asked for one more tool after the
/// budget ran out.
///
/// The unrun directive is REPLACED rather than left in place. Returning it
/// verbatim would put a line that looks exactly like a tool call into the
/// task's output, where it becomes a dependency's context and gets imitated
/// by the next agent — a phantom call nobody ever ran, propagating downstream.
/// The note that takes its place says what happened, in the output, where
/// whoever reads the answer will see it.
pub(super) fn budget_spent(reply: &str, max_rounds: u32) -> String {
    let kept: Vec<&str> = reply.lines().collect();
    let cut = kept
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .unwrap_or(kept.len());
    with_budget_note(&kept[..cut].join("\n"), max_rounds)
}

/// `body` plus the budget note. The native path uses this directly: a
/// structured tool call never lands in the output text, so there is nothing to
/// strip — only the note to add.
pub(super) fn with_budget_note(body: &str, max_rounds: u32) -> String {
    let note = format!(
        "[tool budget spent — {max_rounds} calls for this task; the last request was not run]"
    );
    if body.trim().is_empty() {
        note
    } else {
        format!("{}\n\n{note}", body.trim_end())
    }
}
