//! The last word: one more model call, with the tools taken away, for a
//! worker the budget stopped while it was still asking for them.
//!
//! A worker that spent its rounds reading and asked for one more read used to
//! end on whatever it had written beside that last ask, usually nothing, plus
//! the budget note. Everything it had gathered was in the prompt for a round
//! that never went out, and the swarm's closing answer and its judge were
//! handed an empty task. So that prompt goes out once more, with no tools on
//! it and one instruction at the end: answer now.
//!
//! Both loops send it as ONE plain prompt: the task, every exchange so far,
//! the call that was refused, the instruction. The native loop's turns are
//! written out as text rather than resent as turns, because a request that
//! carries tool_use and tool_result blocks with no tools beside it is one
//! Anthropic refuses, and a refused last word leaves the worker where it was.

#[cfg(test)]
#[path = "lastwordrun_tests.rs"]
mod run_tests;
#[cfg(test)]
#[path = "lastword_tests.rs"]
mod tests;

use std::sync::Arc;

use super::chunks::ChunkSink;
use crate::agent::AgentContext;
use crate::bus::HiveEvent;
use crate::provider::{CompletionRequest, Provider, ToolInvocation, Turn};
use crate::tools::exchanges::Exchange;

/// What the last word's prompt ends on.
pub(super) const INSTRUCTION: &str = "Your tool budget is spent. Answer the task now from \
     what you have gathered; say plainly what you could not check.";

/// The result a refused call shows in the last word's prompt, so the model
/// sees what it was about to do and that it did not happen, rather than a log
/// that stops just short of its own last message.
const NOT_RUN: &str = "not run \u{2014} tool budget spent";

/// The whole request text. `body` is the task WITHOUT a tools section: a
/// syntax the model can no longer use is an invitation to use it anyway.
pub(super) fn prompt(body: &str, done: &str, refused: &str) -> String {
    let log: Vec<&str> = [done, refused]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    format!(
        "{body}\n\nTOOL EXCHANGES SO FAR:\n{}\n\n{INSTRUCTION}",
        log.join("\n\n")
    )
}

/// The calls the budget refused, `(label, args)` each, as exchanges whose
/// result says they did not run; the first carries what the model wrote.
pub(super) fn refused(said: &str, calls: impl IntoIterator<Item = (String, String)>) -> String {
    written(
        said,
        calls
            .into_iter()
            .map(|(label, args)| (label, args, NOT_RUN.to_string())),
    )
}

/// The native loop's turns as the text loop writes its exchanges, every
/// result whole: the native loop resends every turn whole, so that is what
/// its next round would have carried.
pub(super) fn transcript(turns: &[Turn], label: impl Fn(&ToolInvocation) -> String) -> String {
    let mut shown = Vec::new();
    let mut asked: Option<(&str, &[ToolInvocation])> = None;
    for turn in turns {
        match turn {
            Turn::Assistant { text, calls } => asked = Some((text, calls)),
            Turn::ToolResults(results) => {
                let Some((said, calls)) = asked.take() else {
                    continue;
                };
                // The loop answers every call, in order, so the two zip.
                let rounds = calls
                    .iter()
                    .zip(results)
                    .map(|(c, r)| (label(c), c.input.to_string(), r.content.clone()));
                shown.push(written(said, rounds));
            }
            Turn::User(_) => {}
        }
    }
    shown.join("\n\n")
}

/// `(label, args, result)` exchanges, whole, the first under `said`.
fn written(said: &str, rounds: impl Iterator<Item = (String, String, String)>) -> String {
    let shown: Vec<String> = rounds
        .enumerate()
        .map(|(i, (label, args, result))| {
            let said = if i == 0 { said } else { "" };
            Exchange::new(said, &label, &args, result).to_string()
        })
        .collect();
    shown.join("\n\n")
}

/// Send `prompt` once with no tools, billed like any other round. The answer,
/// or `None` when the call failed or said nothing; either way the caller
/// keeps the output it had. The tool directives the reply ends on are cut,
/// as `toolloop::budget_spent` cuts them: nothing is left to run them.
pub(super) async fn ask(
    ctx: &AgentContext,
    provider: &Arc<dyn Provider>,
    model_id: &str,
    system: Option<String>,
    prompt: String,
    max_tokens: u32,
    sink: &ChunkSink,
) -> Option<String> {
    let req = CompletionRequest {
        model: model_id.to_string(),
        system,
        prompt,
        max_tokens,
        ..Default::default()
    };
    let completion = provider
        .complete_streaming(req, Arc::clone(&sink.on_chunk))
        .await
        .ok()?;
    sink.settle(&completion);
    ctx.bus.publish(HiveEvent::TokenDelta {
        agent: ctx.agent.clone(),
        input: completion.input_tokens,
        output: completion.output_tokens,
    });
    ctx.bus.publish(HiveEvent::CostDelta {
        agent: ctx.agent.clone(),
        micros_usd: super::cost::billed(model_id, ctx.task.model, &completion),
    });
    let text =
        crate::tools::split_tool_calls(&completion.text).map_or(completion.text, |(body, _)| body);
    (!text.trim().is_empty()).then(|| text.trim().to_string())
}
