//! The harness: one agent turn, for every engine crew runs.
//!
//! A turn is a model call, and while the reply asks for a tool, the tool run
//! and the model called again with the result, until it answers, fails, is
//! stopped or spends its rounds. crew had written it twice — the swarm's
//! `ApiAgent` and the relay's `run_tools` in the broker — and the two had
//! drifted: one went native while the other stayed on `@tool` lines, they
//! counted a round differently, and each had answers the other lacked
//! (`docs/superpowers/goals/2026-10-24-one-harness.md`). The swarm runs every
//! task on it, and the relay every hop with tools, CLI agents' included.
//!
//! What an engine hands in is a [`TurnContext`] — who is acting, where its
//! events go, the rounds it may draw, the stop it honours — and a
//! [`TurnSpec`], what the turn is for. What it gets back is how the turn
//! [`Ended`]. What is one engine's own stays with that engine: the swarm's
//! dependency results and planner-written system prompt, its checklist per
//! task, and the scheduler's `Attempt` are `ApiAgent`'s, not the harness's;
//! the relay's frame and hop routing are the broker's, handed in as a prompt.
//! What every turn gets, whoever runs it: each reply made whole
//! (`provider::cutoff`), the advisor on a failed check (an [`Advice`],
//! `crate::advisor`), and the lines under an answer — the checklist's, a call
//! that never ran (`finish`).

mod chunks;
mod cost;
mod failure;
mod finish;
mod lastword;
mod native;
mod note;
mod overflow;
mod pending;
mod steered;
mod stubs;
mod text;
mod textround;
mod toolloop;

#[cfg(test)]
#[path = "advice_tests.rs"]
mod advice_tests;
#[cfg(test)]
#[path = "whole_tests.rs"]
mod whole_tests;

pub(crate) use cost::billed;
pub(crate) use failure::reason;

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::ModelTier;
use crate::provider::cutoff::Whole;
use crate::provider::Provider;
use crate::steers::Steers;
use crate::tools::budget::ToolBudget;
use crate::tools::{ToolCall, ToolCatalog, Tools};

/// Where a turn's events go: the swarm's bus, or whatever an engine that has
/// no bus feeds them to. Best effort, like [`EventBus::publish`]: a turn
/// never waits on whoever is listening.
pub trait Events: Send + Sync {
    fn publish(&self, ev: HiveEvent);
}

impl Events for EventBus {
    fn publish(&self, ev: HiveEvent) {
        EventBus::publish(self, ev);
    }
}

/// What a turn is told about a round of tools beyond their results: the
/// advisor, a second model reading a failed check (`crate::advisor`). What it
/// says joins the turn after the round's results, and stands until it says
/// something else.
pub trait Advice: Send + Sync {
    /// One round's `calls` and how each came back, `(ok, text)`, in order:
    /// the section the turn carries from here on, or empty for none. Read on
    /// the turn's own thread, between rounds, never once it is stopped, and
    /// dropped unfinished when the stop comes while it reads. Async: a swarm
    /// runs every worker on one thread, and a read that blocked it would
    /// stop them all.
    fn read<'a>(
        &'a self,
        calls: &'a [ToolCall],
        ran: &'a [(bool, String)],
    ) -> std::pin::Pin<Box<dyn Future<Output = String> + Send + 'a>>;
}

/// Who a turn is, and what it answers to while it runs.
#[derive(Clone)]
pub struct TurnContext {
    /// The id every event of the turn is published under.
    pub agent: AgentId,
    pub events: Arc<dyn Events>,
    /// The pool of tool rounds the turn draws from, which other turns may
    /// share (the swarm's run). Draw through [`TurnContext::take_round`],
    /// which announces the draw.
    pub budget: ToolBudget,
    /// The stop. A turn looks before each model call and each tool call, and
    /// races a model call in flight against it (`agent::stop`).
    pub cancel: Arc<AtomicBool>,
    /// What the engine reads each round for, when it reads one.
    pub advice: Option<Arc<dyn Advice>>,
}

impl TurnContext {
    /// Publish `ev` where this turn's events go.
    pub fn publish(&self, ev: HiveEvent) {
        self.events.publish(ev);
    }

    /// Draw one tool round from the pool for a turn that has drawn `used`
    /// already (see [`ToolBudget::take`]), and publish the pool's state so
    /// whoever runs it can watch it drain. The one way a turn spends a round:
    /// a draw nobody announced is a pool nobody can show.
    pub fn take_round(&self, used: u32) -> Option<u32> {
        let left = self.budget.take(used);
        let total = self.budget.total();
        self.publish(HiveEvent::ToolBudget {
            used: total - self.budget.left(),
            total,
        });
        left
    }

    /// Whether the turn has been stopped.
    pub fn stopped(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// `work`'s output, or `None` when the turn is stopped first — `work`
    /// dropped unfinished (see `agent::stop::unless`).
    pub async fn unless_stopped<F: Future>(&self, work: F) -> Option<F::Output> {
        crate::agent::stop::unless(&self.cancel, work).await
    }

    /// What the turn's [`Advice`] makes of a round, or `kept` (what it said
    /// last) when it has none or the turn was stopped, before or during the
    /// read.
    pub(crate) async fn advised(
        &self,
        calls: &[ToolCall],
        ran: &[(bool, String)],
        kept: &str,
    ) -> String {
        match &self.advice {
            Some(a) if !self.stopped() => self
                .unless_stopped(a.read(calls, ran))
                .await
                .unwrap_or_else(|| kept.to_string()),
            _ => kept.to_string(),
        }
    }
}

/// What a turn is for.
#[derive(Clone, Debug)]
pub struct TurnSpec {
    /// The ask as written, which the tools shown are chosen for
    /// (`Tools::specs_for`, `hint_for`, `note_for`).
    pub ask: String,
    /// The first prompt whole: the ask, and whatever the engine adds to it
    /// (the swarm: what the task's dependencies found). No `@tool` hint: on
    /// the native loop the tools are on the wire, and naming the text
    /// convention beside them invites a model to use both.
    pub prompt: String,
    /// The same prompt with the `@tool` hint already in place, for an engine
    /// whose prompt has a place for it (the relay's frame: inside its TASK,
    /// above HOW TO REPLY). Only the text loop reads it; `None` puts the hint
    /// at the end of `prompt`.
    pub hinted: Option<String>,
    /// What the prompt can lose when the model refuses its length: the
    /// prompt without it, or `None` when it has nothing to lose (the relay's
    /// recalled turns, crew-plugin `recall::cut`). Applied once, with the
    /// turn's own cut of its tool results (`overflow`).
    pub shed: Option<Shed>,
    /// How the engine's prompt says to end an answer, pointed at in every
    /// follow-up and in the last word so neither can disagree with it (the
    /// relay: `end it as HOW TO REPLY says`, its `@done` / `@next` line), or
    /// `None` for an answer that ends however it ends.
    pub ending: Option<String>,
    /// How long one model call may take, the cut-off continuation and the
    /// retries of a broken or empty reply included (`provider::cutoff`):
    /// the relay's per-call timeout, or `None` for the provider's own.
    pub call_timeout: Option<std::time::Duration>,
    pub system: Option<String>,
    /// The model id every request names.
    pub model: String,
    /// What a reply is billed at when its model has no price of its own
    /// (`cost::billed`).
    pub tier: ModelTier,
    pub max_tokens: u32,
}

/// A prompt with what it can live without cut out, or `None` (`TurnSpec::shed`).
pub type Shed = fn(&str) -> Option<String>;

/// How a turn ended. Each has already been published by the time it is
/// returned: an answer as the agent's `OutputChunk`, a failure as `Failed`.
#[derive(Clone, Debug, PartialEq)]
pub enum Ended {
    /// The turn's answer.
    Answered(String),
    /// A provider error ended it: the reason, in the words the pane shows,
    /// and whether it passes (`ProviderError::is_transient`), so the same
    /// turn run again in a moment may well succeed.
    Failed { why: String, transient: bool },
    /// The stop was seen. What the turn's tools did is not undone.
    Stopped,
}

/// Run one turn: call the provider, and while the reply asks for a tool, run
/// it and call the provider again with the result. A provider error ends it,
/// said in the output and marked passing or not (`failure`).
///
/// With no tool surface this is exactly one provider call and the same three
/// events it always published — the loop's first pass IS the old body, and
/// `split_tool_calls` is never even reached.
pub async fn run(
    ctx: TurnContext,
    spec: TurnSpec,
    provider: Arc<dyn Provider>,
    tools: Option<Arc<dyn Tools>>,
    steers: Option<Steers>,
) -> Ended {
    // Every reply whole: continued once when the token ceiling cut it, picked
    // up when its stream broke, asked again when it came back empty.
    let budget = spec
        .call_timeout
        .unwrap_or_else(crate::provider::request_timeout);
    let provider: Arc<dyn Provider> = Arc::new(Whole::new(provider, budget));
    // NATIVE OR TEXT, decided once. Native needs BOTH halves — schemas from
    // the tool surface and tool support from the provider — and when either
    // is missing the `@tool` convention still works, which is why it stays
    // rather than being replaced.
    if let Some(runner) = tools.clone() {
        let specs = runner.specs_for(&spec.ask);
        if !specs.is_empty() && provider.supports_tools() {
            let sink = chunks::ChunkSink::new(&ctx);
            // No tools hint in the prompt: the tools are on the wire, and
            // advertising the text convention beside them invites a model to
            // use both. What was left OFF the wire is the one thing said in
            // prose (`Tools::note_for`, `note.rs`).
            let (system, prompt) =
                note::noted(spec.system, spec.prompt, runner.note_for(&spec.ask));
            let spec = TurnSpec {
                system,
                prompt,
                ..spec
            };
            let catalog = ToolCatalog::build(&specs);
            return native::run(ctx, spec, provider, runner, catalog, sink, steers).await;
        }
    }
    text::run(ctx, spec, provider, tools, steers).await
}
