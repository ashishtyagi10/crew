//! A narrated call is run, not imagined.
//!
//! The exchange log shows every earlier call as `CALLED <tool> <args>` with
//! its output under `RESULT:` (`exchanges`). A model copying that shape wrote
//! its next calls AND their results: live (2026-10-20, qwen-max, a rename
//! across a crate) the answer held four `CALLED sys:edit …` lines, each over
//! an invented `RESULT: edited …`, and ended "all occurrences renamed" and
//! "tests ran". None of it had run, and three uses were left. A `RESULT:`
//! line straight under a call is never one the model was given, so the reply
//! is cut there: the call above it becomes the reply's last word and runs,
//! and the invented output, with all that was written on the strength of it,
//! is dropped.
use std::borrow::Cow;

/// `reply` up to the end of the first call that an invented `RESULT:` line
/// follows; the reply as written when no call is followed by one.
pub(super) fn cut(reply: &str) -> Cow<'_, str> {
    let lines: Vec<&str> = reply.lines().collect();
    for at in (0..lines.len()).filter(|&i| super::directive(lines[i]).is_some()) {
        let Some((_, used)) = super::call_at(&lines, at) else {
            continue;
        };
        let end = at + 1 + used;
        if lines.get(end).is_some_and(|l| invented(l)) {
            return Cow::Owned(lines[..end].join("\n"));
        }
    }
    Cow::Borrowed(reply)
}

/// The exchange log's result marker, as a model writing one out spells it.
fn invented(line: &str) -> bool {
    let t = line.trim();
    t == "RESULT:" || t.starts_with("RESULT: ")
}

#[cfg(test)]
#[path = "narrated_tests.rs"]
mod tests;
