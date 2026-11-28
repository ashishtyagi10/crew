//! The "rounds are running low" line a native tool result carries.
//!
//! The budget used to reach the model only once it was spent (`toolloop`'s
//! budget note), so an agent mid-edit was cut off instead of wrapping up. This
//! tells it two rounds ahead, while a verify step still fits.
//!
//! It is appended to the provider-bound [`ToolOutcome`]s only, AFTER the batch
//! published its per-call `ToolResult` events (the pane's transcript) from
//! their own text, so the host never sees it. It lives in the newest
//! `Turn::ToolResults` and stays there: every earlier turn is resent
//! byte-for-byte, which keeps the provider's prompt-cache prefix stable, and
//! every wire (Anthropic `tool_result`, OpenAI-style `tool` messages) renders
//! those same outcomes, so none needs its own copy.

use crate::provider::ToolOutcome;

/// The line for a round with `left` rounds remaining after it, in a budget of
/// `total`, or none while there is room. Low is `left <= 2` or the last fifth
/// of a big budget; a budget of three or fewer is all low, so says nothing.
/// Not at 0: the line is the warning ahead of the last round, and the spent
/// budget has its own note (`toolloop::with_budget_note`).
pub(super) fn line(left: u32, total: u32) -> Option<String> {
    if total <= 3 || left == 0 || left > 2.max(total / 5) {
        return None;
    }
    let s = if left == 1 { "" } else { "s" };
    Some(format!(
        "[{left} tool round{s} left \u{2014} finish and verify the change, or say plainly what is left undone.]"
    ))
}

/// Add the reminder (if any) to the last outcome of the round just run.
pub(super) fn append(results: &mut [ToolOutcome], left: u32, total: u32) {
    if let (Some(line), Some(last)) = (line(left, total), results.last_mut()) {
        last.content = format!("{}\n\n{line}", last.content.trim_end());
    }
}
