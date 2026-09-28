//! What an agent wrote alongside a tool call, carried into the next prompt.
//!
//! Both engines rebuild the whole prompt after every tool round, and the
//! rebuilt prompt listed each round as its CALLED line and RESULT, nothing
//! else. The text the agent wrote above the call — "the bug is in route.rs;
//! reading clip() next, then its tests" — was gone by the time the result came
//! back, so the agent saw a bare list of reads, had to guess why it made them,
//! and often made one of them again. Claude Code keeps an assistant's text
//! blocks between tool calls in its transcript for the same reason: the
//! reasoning that picked a call is part of what the call means.

/// Chars of one round's message the next prompt carries.
///
/// Enough for the few sentences a model writes on its way to a call. A model
/// that wrote an essay above one pays for it once in its own reply; carrying
/// it whole into every later prompt of the turn would pay for it again each
/// round, and the result below it is what the next round is for.
const SAID_CLIP: usize = 800;

/// The `YOUR MESSAGE:` block for one exchange, ending in a newline so its
/// `CALLED` line can follow directly; empty when the reply was only the call,
/// since a heading over nothing tells the agent nothing.
///
/// `text` is the reply with its call removed (`split_tool_call`), trimmed
/// here and clipped to [`SAID_CLIP`] on a line boundary, with a note saying
/// how much was left out so the agent does not take a cut for an ending.
pub fn said(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    format!("YOUR MESSAGE:\n{}\n", clip_lines(text, SAID_CLIP))
}

/// `text` cut to at most `max` chars at the end of a line, plus a note.
fn clip_lines(text: &str, max: usize) -> String {
    let total = text.chars().count();
    if total <= max {
        return text.to_string();
    }
    let head = head_lines(text, max);
    let left = total - head.chars().count();
    format!("{head}\n[\u{2026} {left} more chars of this message not shown]")
}

/// The whole lines of `text` that fit in `max` chars, trailing space trimmed.
///
/// Whole lines, because a line cut mid-word reads as a thought the model
/// never finished. A first line longer than `max` on its own is the one case
/// with no line to end on, and is cut at `max` chars. Shared with the
/// shortened results of older exchanges (`exchanges`), which are cut the same
/// way for the same reason.
pub(super) fn head_lines(text: &str, max: usize) -> String {
    let mut kept = 0;
    let mut end = 0;
    for line in text.split_inclusive('\n') {
        let n = line.chars().count();
        if kept + n > max {
            break;
        }
        kept += n;
        end += line.len();
    }
    if end == 0 {
        text.chars().take(max).collect()
    } else {
        text[..end].trim_end().to_string()
    }
}

#[cfg(test)]
#[path = "said_tests.rs"]
mod tests;
