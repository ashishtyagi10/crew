//! A relay hop on the harness: an agent's whole turn — its model calls and
//! every tool round between them — run by `crew_hive::harness`, the loop
//! every swarm task runs on. The relay had written the turn a second time,
//! and the two had drifted (`docs/superpowers/goals/2026-10-24-one-harness.md`);
//! it now has none of its own.
//!
//! The agent is the model the turn calls (`model`). An API agent's provider
//! gets native tool-use when it speaks it: each tool's JSON Schema on the
//! wire, the arguments checked by the provider, a call whose arguments were
//! not JSON refused with the reason. A CLI agent is a model with no tool-use,
//! so the `@tool` lines it prints are the text loop's to run, and the tools
//! it runs itself stay its own. Both get the last word when their rounds run
//! out.
//!
//! What is the relay's own stays the broker's and is handed in: the frame,
//! its tools hint in place (`TurnSpec::hinted`) and its recalled turns the
//! first thing to go when the model refuses the length (`TurnSpec::shed`);
//! the pane, as [`Pane`], the hop's stream behind the harness's events; what
//! the user types mid-turn (`steer`); and where the advisor's model comes
//! from (`advise`). Hop routing is untouched: the answer comes back to
//! `Broker::run` as the hop's reply (`reply`).
use std::sync::Arc;

use crew_hive::advisor::Advisor;
use crew_hive::harness::{self, TurnContext, TurnSpec};
use crew_hive::steers::Steers;
use crew_hive::tools::budget::ToolBudget;
use crew_hive::{AgentId, ModelTier, Provider, Spent};

use super::adapter::{Adapter, HopStream, Usage};
use super::hop::{back, Hop, HopKind, RunStats};
use super::{Broker, Envelope};

#[path = "hopmodel.rs"]
mod model;
#[path = "hoppane.rs"]
mod pane;
#[path = "hopreply.rs"]
mod reply;

use pane::{lock, Pane, Shared};

/// Output token ceiling per model call of a CLI agent's turn: unread, since
/// a CLI picks its own (`ApiAdapter`'s is its engine's).
const CLI_MAX_TOKENS: u32 = super::apiadapter::MAX_TOKENS;

impl Broker {
    /// One hop of `agent` with this broker's tools, run whole on the
    /// harness: its reply and usage for routing, or `None` when the hop ended
    /// on an error card (sunk here), as a dial that failed. `prompt` is the
    /// frame without its tools hint, `hinted` with it, and `ask` the message
    /// the tools are chosen for.
    #[allow(clippy::too_many_arguments)] // engine-loop plumbing, one call site
    pub(super) fn harnessed(
        &self,
        agent: &Arc<dyn Adapter>,
        env: &Envelope,
        ask: &str,
        prompt: String,
        hinted: String,
        stats: &mut RunStats,
        stream: &HopStream,
        sink: &mut dyn FnMut(Hop),
    ) -> Option<(String, Usage)> {
        let runner = self.tools.clone()?;
        let tally: Shared = Arc::default();
        let engine = agent.engine();
        // A CLI's calls block on a thread of their own (`model::Spoken`);
        // this runtime only drives the turn around them.
        let own;
        let rt = match &engine {
            Some(e) => e.rt,
            None => match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => {
                    own = rt;
                    &own
                }
                Err(e) => {
                    sink(back(
                        env,
                        HopKind::Error,
                        format!("no runtime for the turn: {e}"),
                    ));
                    return None;
                }
            },
        };
        let (provider, model, system, max_tokens): (Arc<dyn Provider>, _, _, _) = match &engine {
            Some(e) => (
                Arc::new(model::Bounded {
                    inner: Arc::clone(e.provider),
                    model: e.model.to_string(),
                    timeout: self.timeout,
                    tally: Arc::clone(&tally),
                }),
                e.model.to_string(),
                e.system.map(str::to_string),
                e.max_tokens,
            ),
            None => (
                Arc::new(model::Spoken {
                    agent: Arc::clone(agent),
                    timeout: self.timeout,
                    lane: Arc::clone(&stream.on_tool),
                    tally: Arc::clone(&tally),
                }),
                agent.model().to_string(),
                None,
                CLI_MAX_TOKENS,
            ),
        };
        // The advisor reads a failed check (`crew_hive::advisor`); what it
        // costs joins the thread's sums.
        let billed = Arc::clone(&tally);
        let advisor = Advisor::new(
            env.body.clone(),
            Arc::clone(&runner),
            super::advise::find(),
            super::advise::hooks(Arc::new(move |s: Spent| {
                lock(&billed).stats.spend(&s);
            })),
        );
        let ctx = TurnContext {
            agent: AgentId::minted(&env.to),
            events: Arc::new(Pane {
                stream: stream.clone(),
                tally: Arc::clone(&tally),
            }),
            budget: ToolBudget::turn(self.tool_rounds),
            cancel: self.stop_flag(),
            advice: Some(Arc::new(advisor)),
        };
        let spec = TurnSpec {
            ask: ask.to_string(),
            prompt,
            hinted: Some(hinted),
            // The recalled turns are the frame's to lose first.
            shed: Some(super::recall::cut),
            // How to finish is the frame's HOW TO REPLY, pointed at, never
            // restated: alone it is `@done`, with peers `@next` or `@done`.
            ending: Some("end it as HOW TO REPLY says".into()),
            // A reply's continuation and retries fit the hop's own timeout.
            call_timeout: Some(self.timeout),
            system,
            model,
            // Billing's fallback only, and the relay bills from the model's usage.
            tier: ModelTier::Standard,
            max_tokens,
        };
        let steers = Steers::new(super::steer::take).with_tail(super::steer::TAIL);
        let ended = rt.block_on(harness::run(
            ctx,
            spec,
            provider,
            Some(runner),
            Some(steers),
        ));
        (stream.on_thought)(""); // the hop is over: flush the gate's tail
        let tally = std::mem::take(&mut *lock(&tally));
        stats.merge(&tally.stats);
        reply::of(ended, tally, env, sink)
    }

    /// The stop a turn on the harness honours: this broker's `/stop`, else
    /// the running task's (`cancelscope`), else one nobody will set.
    fn stop_flag(&self) -> Arc<std::sync::atomic::AtomicBool> {
        self.cancel
            .clone()
            .or_else(super::cancelscope::current)
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "hopcli_tests.rs"]
mod cli_tests;
#[cfg(test)]
#[path = "hopend_tests.rs"]
mod end_tests;
#[cfg(test)]
#[path = "hopkit_tests.rs"]
mod kit;
#[cfg(test)]
#[path = "hopturn_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "hopfull_tests.rs"]
mod full_tests;
