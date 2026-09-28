//! What a turn spent before its arm began, folded into the total the arm says.
//!
//! WHY: the pane's token and cost meters, and the 5h/7d windows behind them
//! (the app's `usageledger`, fed from the same event), read ONE `Stats` per
//! turn — the one with no agent — and every arm builds that total from its
//! own calls. The router picks the shape before any arm runs, so its call
//! was in no arm's sum: every routed turn read low by one call, a relay
//! reply and a swarm alike. The arm's events pass through here, and the
//! first turn total among them carries the router's cost as well. An arm
//! that says no total at all still gets one, alone, when it ends: the call
//! was made, and a meter that forgot it would be wrong in the same way.
use crew_hive::Spent;

use crate::protocol::PluginEvent;

/// Run `arm` with its events passed on to `emit`, `spent` added to the first
/// turn-total `Stats` it says (never to an agent's reply stat — the router
/// is not the agent that replied), or said on its own after the arm when it
/// said none. Nothing spent passes every event through untouched.
pub(crate) fn folded(
    spent: Spent,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
    arm: impl FnOnce(&mut dyn FnMut(PluginEvent) -> anyhow::Result<()>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut left = (!spent.is_zero()).then_some(spent);
    let ran = arm(&mut |mut ev| {
        if let Some(s) = left {
            if add(&mut ev, s) {
                left = None;
            }
        }
        emit(ev)
    });
    // An arm that failed may still have spent the router's call: it is said
    // either way, and the arm's own error is the one returned.
    let Some(s) = left else { return ran };
    let said = emit(total(s));
    ran.and(said)
}

/// Add `s` to `ev` when it is a turn total; whether it was.
fn add(ev: &mut PluginEvent, s: Spent) -> bool {
    match ev {
        PluginEvent::Stats {
            agent,
            tokens,
            tok_in,
            tok_out,
            cost_microusd,
            ..
        } if agent.is_empty() => {
            *tokens += s.tokens();
            *tok_in += s.input;
            *tok_out += s.output;
            *cost_microusd += s.micros_usd;
            true
        }
        _ => false,
    }
}

/// A turn total of `s` alone: no exchange with an agent, no tool pool.
fn total(s: Spent) -> PluginEvent {
    PluginEvent::Stats {
        exchanges: 0,
        tokens: s.tokens(),
        agent: String::new(),
        ms: 0,
        ctx: 0,
        tok_in: s.input,
        tok_out: s.output,
        cost_microusd: s.micros_usd,
        tools: None,
    }
}

#[cfg(test)]
#[path = "turnspent_tests.rs"]
mod tests;
