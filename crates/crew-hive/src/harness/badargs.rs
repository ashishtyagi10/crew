//! A call whose arguments were not JSON is answered, not run.
//!
//! It used to run with `{}`: the tool said "missing argument path", which is
//! true and useless, because the model had sent a path, inside JSON that was
//! cut off or slipped. So the model sent the same broken call again. Now it
//! is told what the parser said and shown the start of what it sent, and when
//! its reply hit the token limit (the usual cause: an edit carrying a whole
//! file), that the call has to be smaller, not merely fixed.

use crate::provider::{ToolInvocation, ToolOutcome};

/// Added when the reply that carried the call was cut off at `max_tokens`.
/// Retrying the same call would be cut off at the same place.
const CUT: &str = "the reply was cut off at the token limit; \
                   send a shorter call (e.g. sys:edit with a smaller fragment)";

/// The answer to `call` when its arguments did not parse, or `None` to run
/// it. `cut`: the reply it came in stopped at the token limit.
pub(super) fn refusal(call: &ToolInvocation, cut: bool) -> Option<ToolOutcome> {
    let why = call.bad_args.as_deref()?;
    let mut content = format!("your arguments were not valid JSON: {why}");
    if cut {
        content.push('\n');
        content.push_str(CUT);
    }
    Some(ToolOutcome {
        id: call.id.clone(),
        name: call.name.clone(),
        content,
        is_error: true,
    })
}

#[cfg(test)]
#[path = "badargs_tests.rs"]
mod tests;
