//! Where a worker's output becomes final, on either loop: an answer with no
//! tool asked for, or the one written after the budget ran out.
//!
//! One place, because it is where the checklist's last word goes (`todo`): a
//! worker that kept a list and left steps on it says which under its answer,
//! where the closing answer and the pane both read it.

use std::sync::Arc;

use super::{Ended, TurnContext};
use crate::bus::HiveEvent;
use crate::tools::{todo, Tools};

/// `text` as the turn's answer: published as the output chunk and returned,
/// with the unfinished line under it when `tools` kept a list.
pub(super) fn answer(ctx: &TurnContext, tools: Option<&Arc<dyn Tools>>, text: String) -> Ended {
    let text = todo::finished(text, tools.and_then(|t| t.checklist()));
    ctx.publish(HiveEvent::OutputChunk {
        agent: ctx.agent.clone(),
        text: text.clone(),
    });
    Ended::Answered(text)
}
