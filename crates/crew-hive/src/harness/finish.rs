//! Where a worker's output becomes final, on either loop: an answer with no
//! tool asked for, or the one written after the budget ran out.
//!
//! One place, because it is where the engine's last words go: a worker that
//! kept a list and left steps on it says which under its answer (`todo`), and
//! one whose answer still holds a call says it never ran (`never_ran`), where
//! the closing answer, the judge and the pane all read them.

use std::sync::Arc;

use super::{Ended, TurnContext};
use crate::bus::HiveEvent;
use crate::tools::{todo, Tools};

/// `text` as the turn's answer: published as the output chunk and returned,
/// with the unfinished line under it when `tools` kept a list, and the
/// never-ran line when a call is left in it.
pub(super) fn answer(ctx: &TurnContext, tools: Option<&Arc<dyn Tools>>, text: String) -> Ended {
    let text = todo::finished(text, tools.and_then(|t| t.checklist()));
    let text = match tools {
        Some(_) => crate::tools::never_ran(text),
        // No tools, no calls: a `@tool` line is just text (the keyless run's
        // answer stays byte for byte what it was).
        None => text,
    };
    ctx.publish(HiveEvent::OutputChunk {
        agent: ctx.agent.clone(),
        text: text.clone(),
    });
    Ended::Answered(text)
}
