//! Prompt assembly for the `@tool` loop's rounds (`text.rs`).
//!
//! Split out of the loop so the loop stays readable: this module is pure
//! string work and is tested as such, with no provider, bus or runtime in
//! sight.

#[cfg(test)]
#[path = "toolloop_tests.rs"]
mod tests;

use super::native::MAX_CALLS_PER_TURN;
use crate::tools::exchanges::{Exchange, Exchanges};

/// Cap on one tool result fed back into the next prompt, in chars.
///
/// Mirrors the relay's 6k. A tool that returns a 200 KB JSON page — an API
/// listing, a directory, a file read — would otherwise blow the next request
/// past the model's context and fail the task at the exact moment the tool
/// SUCCEEDED, which is the most confusing failure this loop can produce.
///
/// Cut from the MIDDLE (`clip_middle`), both loops: a build or a test run
/// says what went wrong at its end, and `sys:read_file` says where to go on
/// on its last line. The swarm's clip kept the head and the relay's kept the
/// last line, two rules for one cap; the head says what a result is about
/// and the tail what it found. `sys:read_file` pages are sized to pass whole
/// (crew-plugin's `sysread::PAGE`, 5,600 bytes plus a one-line note).
pub(super) const RESULT_CAP: usize = 6_000;

/// `s` held to `max` chars, its middle cut (`clip_middle`).
pub(super) fn clip(s: &str, max: usize) -> String {
    crate::clipmiddle::clip_middle(s, max)
}

/// One round as the next prompt shows it to the agent: what it wrote with
/// the call (`said`, the reply with the call cut out; see
/// [`crate::tools::said`]), then the CALLED/RESULT pair.
pub(super) fn exchange(said: &str, label: &str, args: &str, result: &str) -> Exchange {
    Exchange::new(said, label, args, clip(result, RESULT_CAP))
}

/// The prompt for the round after a tool ran: the original task (tools hint
/// included, so a second call is still spellable), the task's checklist
/// (`list`, empty unless it keeps one), then every exchange so far, the older
/// ones with their results shortened ([`Exchanges::render`]). The list goes
/// ABOVE the exchanges, where no shortening reaches it; what the user added
/// mid-run (`heard`, `steered`) goes after them, where the newest word
/// belongs, ahead of the budget line.
///
/// `rounds_left` — round trips, every call of a reply in one — is stated to
/// the agent rather than merely enforced. A budget
/// an agent cannot see is one it plans straight past, and then the task ends
/// mid-sequence with a tool call nobody ran — the relay learned this the
/// expensive way and the wording is carried over deliberately.
///
/// One sentence per case. "You may call another tool" used to follow every
/// budget line, "This was your LAST tool call" included, so the last round
/// was told both to stop calling and that it could call again.
pub(super) fn follow_up(
    base: &str,
    list: &str,
    exchanges: &Exchanges,
    heard: &str,
    rounds_left: u32,
) -> String {
    let next = match rounds_left {
        0 => "That was your LAST round of tool calls for this task: answer now with what \
              you have."
            .to_string(),
        n => format!(
            "You may make {n} more round(s) of tool calls for this task, up to \
             {MAX_CALLS_PER_TURN} calls in one reply, or give your final answer."
        ),
    };
    format!(
        "{base}\n\n{list}TOOL EXCHANGES SO FAR:\n{}\n\n{heard}{next}",
        exchanges.render()
    )
}

/// What an agent's output becomes when the budget ran out and the last word
/// said nothing: the reply that asked for the last batch, its calls cut.
///
/// The directives are REPLACED rather than left in place. Returning them
/// verbatim would put a line that looks exactly like a tool call into the
/// task's output, where it becomes a dependency's context and gets imitated
/// by the next agent — a call that ran, but whose result nobody read,
/// propagating downstream. The note that takes its place says what happened,
/// in the output, where whoever reads the answer will see it.
///
/// The WHOLE call goes, every call of the reply, as the parser found them. Cutting the last line was
/// enough when a call had to be one line; a fenced call with its JSON over
/// six lines lost only its closing fence, and the rest reached the output.
pub(super) fn budget_spent(reply: &str, max_rounds: u32) -> String {
    let body = crate::tools::split_tool_calls(reply).map_or_else(|| reply.to_owned(), |(b, _)| b);
    with_budget_note(&body, max_rounds)
}

/// `body` plus the budget note. The native path uses this directly: a
/// structured tool call never lands in the output text, so there is nothing to
/// strip — only the note to add.
pub(super) fn with_budget_note(body: &str, max_rounds: u32) -> String {
    let note = format!("[tool budget spent \u{2014} {max_rounds} rounds of tool calls]");
    if body.trim().is_empty() {
        note
    } else {
        format!("{}\n\n{note}", body.trim_end())
    }
}
