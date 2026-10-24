//! The harness: one agent turn, for every engine crew runs.
//!
//! A turn is a model call, and while the reply asks for a tool, the tool run
//! and the model called again with the result, until it answers, fails, is
//! stopped or spends its rounds. crew had written it twice — the swarm's
//! `ApiAgent` and the relay's `run_tools` in the broker — and the two had
//! drifted: one went native while the other stayed on `@tool` lines, they
//! counted a round differently, and each had answers the other lacked
//! (`docs/superpowers/goals/2026-10-24-one-harness.md`). This is the one the
//! swarm runs; the relay moves onto it next.
//!
//! What an engine hands in is a [`TurnContext`] — who is acting, where its
//! events go, the rounds it may draw, the stop it honours — and a
//! [`TurnSpec`], what the turn is for. What it gets back is how the turn
//! [`Ended`]. What is one engine's own stays with that engine: the swarm's
//! dependency results and planner-written system prompt, its checklist per
//! task, and the scheduler's `Attempt` are `ApiAgent`'s, not the harness's.

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

pub(crate) use cost::billed;
pub(crate) use failure::reason;

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::ModelTier;
use crate::provider::Provider;
use crate::steers::Steers;
use crate::tools::budget::ToolBudget;
use crate::tools::{ToolCatalog, Tools};

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
}

/// What a turn is for.
#[derive(Clone, Debug)]
pub struct TurnSpec {
    /// The ask as written, which the tools shown are chosen for
    /// (`Tools::specs_for`, `hint_for`, `note_for`).
    pub ask: String,
    /// The first prompt whole: the ask, and whatever the engine adds to it
    /// (the swarm: what the task's dependencies found).
    pub prompt: String,
    pub system: Option<String>,
    /// The model id every request names.
    pub model: String,
    /// What a reply is billed at when its model has no price of its own
    /// (`cost::billed`).
    pub tier: ModelTier,
    pub max_tokens: u32,
}

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
