//! Who a turn is and what it answers to: the [`TurnContext`] an engine hands
//! [`super::run`], where the turn's [`Events`] go, and the [`Advice`] it
//! reads each round for.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::tools::budget::ToolBudget;
use crate::tools::ToolCall;

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
