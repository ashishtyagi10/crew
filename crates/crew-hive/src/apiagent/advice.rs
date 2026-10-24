//! Each task's advisor (`crate::advisor`): its model found where the swarm
//! says, its line said the swarm's way, and every read it makes billed on
//! the run's bus.

use std::sync::Arc;

use crate::advisor::{Advisor, Find, Hooks};
use crate::agent::AgentContext;
use crate::bus::HiveEvent;
use crate::tools::Tools;

/// How an advisor says its one quiet line (`advisor::Hooks::say`).
pub type Say = Arc<dyn Fn(&str) + Send + Sync>;

/// The advisor for `ctx`'s task on `tools`, billing each read on the run's
/// bus under the task's agent, where the swarm's sums and its budget
/// governor read every worker's spend.
pub(super) fn advisor_for(
    ctx: &AgentContext,
    tools: &Arc<dyn Tools>,
    (find, say): &(Find, Say),
) -> Advisor {
    let (bus, agent) = (ctx.bus.clone(), ctx.agent.clone());
    let spent = Arc::new(move |s: crate::spent::Spent| {
        bus.publish(HiveEvent::TokenDelta {
            agent: agent.clone(),
            input: u32::try_from(s.input).unwrap_or(u32::MAX),
            output: u32::try_from(s.output).unwrap_or(u32::MAX),
        });
        bus.publish(HiveEvent::CostDelta {
            agent: agent.clone(),
            micros_usd: s.micros_usd,
        });
    });
    let hooks = Hooks {
        say: Arc::clone(say),
        spent,
    };
    Advisor::new(
        ctx.task.prompt.clone(),
        Arc::clone(tools),
        Arc::clone(find),
        hooks,
    )
}
