//! The planner's one second chance: a re-ask when the reply was not a task
//! array.
//!
//! WHY: the relay already re-asks an agent that forgot its control line
//! (`route::repair_prompt` in the broker). The planner had no such grace —
//! one sloppy reply, and the whole swarm degraded to a single direct answer
//! — and the planner's reply is the one that decides whether there IS a
//! swarm. Bounded to ONE re-ask: a model that fails twice will not be argued
//! into JSON, and every retry costs a full planning call.
//!
//! The re-ask is observable twice over: a line on stderr (the broker's log)
//! when it happens, and the surfaced error naming it when it did not help —
//! so the pane's "planning failed (…)" says how hard the planner tried.
use super::{extract, parse_plan, PlanError};
use crate::graph::TaskGraph;
use crate::provider::{CompletionRequest, Provider};

/// Most of the failed reply that the re-ask shows back to the model. The
/// reply is already bounded by `max_tokens`; this only keeps a pathological
/// one from doubling the planning bill.
const REPLY_CAP: usize = 4000;

/// Plan with `req`, and if the reply does not parse, once more with the
/// error shown. Provider errors are not a reply and are never re-asked.
pub(crate) async fn plan_with_repair<P: Provider>(
    provider: P,
    req: CompletionRequest,
) -> Result<TaskGraph, PlanError> {
    let first = provider.complete(req.clone()).await?;
    let err = match graph_from(&first.text) {
        Ok(graph) => return Ok(graph),
        Err(e) => e,
    };
    eprintln!("crew: the planner's reply was not a task array ({err}) \u{2014} asking once more");
    let retry = CompletionRequest {
        prompt: repair_prompt(&req.prompt, &first.text, &err),
        ..req
    };
    let second = provider.complete(retry).await?;
    graph_from(&second.text).map_err(|e| match e {
        PlanError::Parse(s) => PlanError::Parse(format!("{s}; after one repair re-ask")),
        other => other,
    })
}

/// The array inside `text`, tidied, then held to the strict parser.
fn graph_from(text: &str) -> Result<TaskGraph, PlanError> {
    let array = extract::array_text(text)
        .ok_or_else(|| PlanError::Parse("no JSON array in the reply".into()))?;
    parse_plan(&extract::without_trailing_commas(array))
}

/// The original goal, then what went wrong and the reply it went wrong in,
/// then the one instruction the first prompt's last line already gave.
pub(crate) fn repair_prompt(goal: &str, reply: &str, err: &PlanError) -> String {
    let shown: String = reply.chars().take(REPLY_CAP).collect();
    format!(
        "{goal}\n\n\
         Your previous reply could not be read as the task array ({err}):\n\n\
         {shown}\n\n\
         Reply again with ONLY the JSON array \u{2014} no code fence, no prose \
         before or after it, no trailing commas."
    )
}

#[cfg(test)]
#[path = "repair_tests.rs"]
mod tests;
