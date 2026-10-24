//! The native tool-use loop: structured calls on the wire, no `@tool` line.
//!
//! Chosen over [`super::toolloop`]'s text convention whenever the tool surface
//! has schemas AND the provider speaks tools. The difference that matters is
//! not elegance: the model is shown each tool's JSON Schema and the provider
//! validates the arguments, instead of being shown a name and a 100-character
//! description clip and asked to hand-write JSON on the last line of a reply.

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;

#[path = "badargs.rs"]
mod badargs;
#[path = "batch.rs"]
mod batch;

use std::sync::Arc;

use super::{Ended, TurnContext, TurnSpec};
use crate::bus::HiveEvent;
use crate::provider::{CompletionRequest, Provider, ToolInvocation, ToolOutcome, Turn};
use crate::steers::Steers;
use crate::tools::{seen::Seen, todo, ToolCall, ToolCatalog, Tools};

/// Most tools one turn may fire, however many the model asked for.
///
/// Parallel tool use is a real feature — three independent lookups in one turn
/// is exactly what a swarm wants — but "however many it emitted" is not a
/// bound. A model that emits fifty calls in one turn would run fifty shell
/// commands before anything counted a round.
///
/// The text loop holds a reply to the same number (`textround`).
pub(super) const MAX_CALLS_PER_TURN: usize = 8;

/// What a call past [`MAX_CALLS_PER_TURN`] is answered with.
pub(super) fn past_bound() -> String {
    format!("not run \u{2014} at most {MAX_CALLS_PER_TURN} tools per turn")
}

/// What a call resolved to, or why it did not.
fn outcome_for_unknown(call: &ToolInvocation, catalog: &ToolCatalog) -> ToolOutcome {
    let known = catalog.names();
    // Naming the near misses turns an invented name into one recoverable
    // round instead of a repeat of the same wrong guess — and at retrieval
    // scale the whole list would be the wall the picker exists to avoid.
    let tail = match known.contains(&"sys__find_tools") {
        true => "; sys__find_tools searches them",
        false => "",
    };
    ToolOutcome {
        id: call.id.clone(),
        name: call.name.clone(),
        content: crate::tools::near::unknown("tool", &call.name, &known, tail),
        is_error: true,
    }
}

/// Run one turn with native tool use. Returns how it ended; every model call,
/// tool call and failure publishes on `ctx`'s events as it happens.
pub(super) async fn run(
    ctx: TurnContext,
    spec: TurnSpec,
    provider: Arc<dyn Provider>,
    tools: Arc<dyn Tools>,
    catalog: ToolCatalog,
    sink: super::chunks::ChunkSink,
    steers: Option<Steers>,
) -> Ended {
    let agent_id = ctx.agent.clone();
    let TurnSpec {
        prompt,
        shed,
        ending,
        system,
        model: model_id,
        tier,
        max_tokens,
        ..
    } = spec;
    let mut turns: Vec<Turn> = Vec::new();
    let mut round: u32 = 0;
    // Reads this task has made. A repeat is answered from the result already
    // in `turns`, which every request resends whole. So `Seen::check` alone,
    // not `Exchanges::repeat`: the result a pointer names is always in the
    // request beside it, until the turns are cut to fit (`overflow::Cut`).
    let mut seen = Seen::default();
    // What the user said before this worker started ends its prompt; what
    // comes after joins as a turn of its own (`steered`).
    let (mut prompt, mut taken) = super::steered::opening(prompt, steers.as_ref());
    let label = |c: &ToolInvocation| label_of(c, &catalog);
    let mut cut = super::overflow::Cut::new(&prompt, system.as_deref(), &label);
    // What the engine made of the latest round it read (`Advice`).
    let mut advice = String::new();

    loop {
        // A stop pressed while the last round's tools ran: no next call
        // (`agent::stop`). The tools already run are not undone.
        if ctx.stopped() {
            return Ended::Stopped;
        }
        // Between rounds only: before the first, `opening` just asked.
        if !turns.is_empty() {
            turns.extend(super::steered::turn(steers.as_ref(), &mut taken));
        }
        cut.sending(&turns);
        let req = CompletionRequest {
            model: model_id.clone(),
            system: system.clone(),
            prompt: prompt.clone(),
            max_tokens,
            turns: turns.clone(),
            tools: catalog.defs().to_vec(),
        };
        // Raced against the stop: dropping the call aborts the request.
        let call = provider.complete_streaming(req, Arc::clone(&sink.on_chunk));
        let completion = match ctx.unless_stopped(call).await {
            None => return Ended::Stopped,
            Some(Ok(c)) => c,
            Some(Err(err))
                if cut.refused(&ctx, &err, &mut turns, &mut seen, (&mut prompt, shed)) =>
            {
                continue
            }
            Some(Err(err)) => return super::failure::failed(&ctx, &err),
        };
        cut.answered();
        sink.settle(&completion);
        ctx.publish(HiveEvent::TokenDelta {
            agent: agent_id.clone(),
            input: completion.input_tokens,
            output: completion.output_tokens,
        });
        ctx.publish(HiveEvent::CostDelta {
            agent: agent_id.clone(),
            micros_usd: super::cost::billed(&model_id, tier, &completion),
        });

        if completion.calls.is_empty() {
            return super::finish::answer(&ctx, Some(&tools), completion.text);
        }

        // The batch runs whether or not the pool has a round for it: past the
        // budget, its edits and its check are what the answer stands on.
        let drawn = ctx.take_round(round);
        let wrote = tools.checklist().map(todo::Checklist::writes);
        let results = batch::run(&ctx, &tools, &catalog, &mut seen, &completion, round).await;
        let ran = ran_of(&completion.calls, &results, &catalog);
        let said = completion.text.clone();
        turns.push(Turn::Assistant {
            text: completion.text,
            calls: completion.calls,
        });
        turns.push(Turn::ToolResults(results));

        if drawn.is_none() {
            // Everything gathered, that batch included, goes back once more
            // as plain text with no tools on the wire, for an answer
            // (`lastword`); failing that, what it had written beside the ask
            // is all there is. Unlike the text path there is no directive to
            // strip — a structured call never lands in the output.
            let total = ctx.budget.total();
            let last = super::lastword::prompt(
                &prompt,
                &todo::section(tools.checklist()),
                &super::lastword::transcript(&turns, label),
                ending.as_deref().unwrap_or_default(),
            );
            let answer = super::lastword::ask(
                &ctx, &provider, &model_id, tier, system, last, max_tokens, &sink,
            )
            .await;
            if ctx.stopped() {
                return Ended::Stopped;
            }
            let text = super::toolloop::with_budget_note(answer.as_deref().unwrap_or(&said), total);
            return super::finish::answer(&ctx, Some(&tools), text);
        }

        cut.after_round(&mut turns, &mut seen);
        // The checklist as a note of its own, in the rounds that changed it
        // and no other: every turn here is resent whole, so a note sent every
        // round would ride in each request once per round already taken, and
        // the one after the last change is still there to read.
        if let Some(list) = tools.checklist().filter(|l| Some(l.writes()) != wrote) {
            let note = list.section();
            if !note.is_empty() {
                turns.push(Turn::User(note.trim_end().to_string()));
            }
        }
        // What the engine made of the round (`Advice`), as a note of its own
        // when it says something new, for the same reason.
        let (calls, outcomes): (Vec<ToolCall>, Vec<(bool, String)>) = ran.into_iter().unzip();
        let read = ctx.advised(&calls, &outcomes, &advice);
        if read != advice && !read.trim().is_empty() {
            turns.push(Turn::User(read.trim_end().to_string()));
        }
        advice = read;
        round += 1;
    }
}

/// Each call of a round that names a tool, as crew names it, beside how it
/// came back `(ok, text)`: what the engine's `Advice` reads.
fn ran_of(
    calls: &[ToolInvocation],
    results: &[ToolOutcome],
    catalog: &ToolCatalog,
) -> Vec<(ToolCall, (bool, String))> {
    calls
        .iter()
        .zip(results)
        .filter_map(|(c, r)| {
            let (server, tool) = catalog.resolve(&c.name)?;
            let call = ToolCall {
                server: server.to_string(),
                tool: tool.to_string(),
                args: c.input.to_string(),
            };
            Some((call, (!r.is_error, r.content.clone())))
        })
        .collect()
}

/// `server:tool` when the name resolves, else the name the model invented.
fn label_of(call: &ToolInvocation, catalog: &ToolCatalog) -> String {
    match catalog.resolve(&call.name) {
        Some((s, t)) => format!("{s}:{t}"),
        None => call.name.clone(),
    }
}
