//! The `@tool` loop: a turn whose tools are named in the prompt and asked
//! for on the reply's last lines, for a provider with no tool-use on the
//! wire or a tool surface with no schemas.

use std::sync::Arc;

use super::{
    chunks, failure, finish, lastword, overflow, steered, textround, toolloop, Ended, TurnContext,
    TurnSpec,
};
use crate::bus::HiveEvent;
use crate::provider::{CompletionRequest, Provider};
use crate::steers::Steers;
use crate::tools::{self, Tools};

/// Run `spec` as `@tool` rounds until it answers.
pub(super) async fn run(
    ctx: TurnContext,
    spec: TurnSpec,
    provider: Arc<dyn Provider>,
    tools: Option<Arc<dyn Tools>>,
    steers: Option<Steers>,
) -> Ended {
    let agent_id = ctx.agent.clone();
    let TurnSpec {
        ask,
        prompt: body,
        hinted,
        system,
        model: model_id,
        tier,
        max_tokens,
    } = spec;
    // The tools section is part of the BASE prompt, not just the first
    // request: every follow-up re-states it, or an agent that used one tool
    // would find the syntax for the second one gone. Where the engine put it,
    // when it placed it itself.
    let base = hinted.unwrap_or_else(|| {
        tools::augment(
            &body,
            &tools.as_ref().map(|t| t.hint_for(&ask)).unwrap_or_default(),
        )
    });
    // Fragments publish as they arrive. NOTE that with tool rounds the deltas
    // now carry MORE than the final `OutputChunk`: every round's thinking
    // streams, while the chunk is the ANSWER. That is the right split — the
    // intervening rounds are published as their own ToolCall/ToolResult
    // events, so nothing is lost, and a transcript built from chunks stays
    // the answer rather than the working.
    let sink = chunks::ChunkSink::new(&ctx);

    let (mut prompt, _) = steered::opening(base.clone(), steers.as_ref());
    let mut exchanges = tools::exchanges::Exchanges::default();
    let mut round: u32 = 0;
    // The rounds left after the last one, for a follow-up rebuilt to fit the
    // model's context (`overflow`), which is done once.
    let (mut left, mut cut) = (0, false);
    // Reads this turn has made. A repeat is answered from the result already
    // in `exchanges`, while the follow-up still carries it whole
    // (`Exchanges::repeat`); once shortened, it runs again.
    let mut seen = tools::seen::Seen::default();
    // What the engine made of the latest round it read (`Advice`), for every
    // follow-up after it.
    let mut advice = String::new();
    loop {
        // A stop pressed while the last round's tools ran: no next call. The
        // tools already run are not undone (`stop`).
        if ctx.stopped() {
            return Ended::Stopped;
        }
        let req = CompletionRequest {
            model: model_id.clone(),
            system: system.clone(),
            prompt,
            max_tokens,
            ..Default::default()
        };
        // Raced against the stop: a call nobody wants any more is dropped,
        // which aborts the request, rather than waited out.
        let call = provider.complete_streaming(req, Arc::clone(&sink.on_chunk));
        let completion = match ctx.unless_stopped(call).await {
            None => return Ended::Stopped,
            Some(Ok(c)) => c,
            // Too long for the model: once, the log cut to fit.
            Some(Err(err)) if !cut && overflow::tightened(&ctx, &err, &mut exchanges) => {
                cut = true;
                let list = tools::todo::section(tools.as_ref().and_then(|t| t.checklist()));
                let heard = steered::heard(steers.as_ref()) + &advice;
                prompt = toolloop::follow_up(&base, &list, &exchanges, &heard, left);
                continue;
            }
            Some(Err(err)) => return failure::failed(&ctx, &err),
        };
        sink.settle(&completion);
        // Billed per round, as it happens: a run that spends four model calls
        // on tools must not look like one call's worth of tokens to the
        // budget governor watching these events.
        ctx.publish(HiveEvent::TokenDelta {
            agent: agent_id.clone(),
            input: completion.input_tokens,
            output: completion.output_tokens,
        });
        ctx.publish(HiveEvent::CostDelta {
            agent: agent_id.clone(),
            micros_usd: super::cost::billed(&model_id, tier, &completion),
        });

        let asked = tools
            .as_ref()
            .and_then(|_| tools::split_tool_calls(&completion.text));
        let (Some(runner), Some((said, calls))) = (tools.as_ref(), asked) else {
            // No tool asked for: this reply is the answer.
            return finish::answer(&ctx, tools.as_ref(), completion.text);
        };
        // One round for the reply, however many calls it made: a round is a
        // round trip (`textround`). The batch runs either way.
        let drawn = ctx.take_round(round);
        let ran = textround::run(&ctx, runner, &mut seen, &mut exchanges, &said, &calls).await;
        let Some(rounds_left) = drawn else {
            // Past the budget the batch still ran: its edits and its check
            // are what the answer stands on. What it gathered, that batch
            // included, goes back once more, tools hint left out, for an
            // answer (`lastword`); failing that, what it had written beside
            // the ask is all there is.
            let total = ctx.budget.total();
            let last = lastword::prompt(
                &body,
                &tools::todo::section(runner.checklist()),
                &exchanges.render(),
            );
            let answer = lastword::ask(
                &ctx, &provider, &model_id, tier, system, last, max_tokens, &sink,
            )
            .await;
            if ctx.stopped() {
                return Ended::Stopped;
            }
            let text = match answer {
                Some(answer) => toolloop::with_budget_note(&answer, total),
                None => toolloop::budget_spent(&completion.text, total),
            };
            return finish::answer(&ctx, Some(runner), text);
        };
        round += 1;
        left = rounds_left;
        advice = ctx.advised(&calls[..ran.len()], &ran, &advice);
        let list = tools::todo::section(runner.checklist());
        let heard = steered::heard(steers.as_ref()) + &advice;
        prompt = toolloop::follow_up(&base, &list, &exchanges, &heard, left);
    }
}
