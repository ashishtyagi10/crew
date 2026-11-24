//! How hard the work turn's model thinks, chosen per route and per agent.
//!
//! The one-shots asked for `low` (`crew_hive::provider::effort`) but every
//! work turn sent nothing, so a commit message and a code review both ran at
//! the model's default — Opus 5.5 `medium`, Sonnet 5.5 `high` — whatever the
//! router had decided the message was. The router's verdict is a [`Shape`],
//! and three kinds of shape want something other than the default:
//!
//! - `plan` and `review` are judgment the user acts on — a plan approved and
//!   run, findings worst-first — so they think `high`;
//! - `commit` and `standup` restate what is in front of the model (a diff, a
//!   log), so they think `low`;
//! - every other shape (`reply`, `fan`, `loop`, `goal`, `swarm`) is open
//!   work of any size, and `resume` makes no call: the model's own default.
//!
//! A specialist's own `effort` in `.crew/specialists.json` outranks the
//! route: the person who wrote it knows that agent. The route's level rides
//! the routed message's thread as a scope ([`enter`]), the way the task's
//! stop does (`cancelscope`), so the agent's call reads it without every
//! arm in between passing it along. Each wire then sends the level only to
//! a model that takes it (`effort::anthropic_field`, `effort::openai_field`).
use std::cell::Cell;

use crew_hive::Effort;

use super::intent::decision::{Decision, Routing};
use super::intent::Shape;

/// The effort the router's `shape` asks for, or `None` for the default.
pub(crate) fn of_route(shape: Shape) -> Option<Effort> {
    match shape {
        Shape::Plan | Shape::Review => Some(Effort::High),
        Shape::Commit | Shape::Standup => Some(Effort::Low),
        Shape::Reply | Shape::Fan | Shape::Loop | Shape::Goal | Shape::Swarm | Shape::Resume => {
            None
        }
    }
}

thread_local! {
    static ROUTE: Cell<Option<Effort>> = const { Cell::new(None) };
}

/// Puts back the level that was there before, when the routed message ends.
pub(crate) struct Scope(Option<Effort>);

impl Drop for Scope {
    fn drop(&mut self) {
        ROUTE.with(|r| r.set(self.0));
    }
}

/// Make `effort` this thread's route level until the scope drops.
pub(crate) fn enter(effort: Option<Effort>) -> Scope {
    Scope(ROUTE.with(|r| r.replace(effort)))
}

/// The routing's decision, with its level entered for as long as the
/// returned scope lives — the routed message's whole run.
pub(crate) fn decided(routing: &Routing) -> (Decision, Scope) {
    let d = routing.decision();
    let scope = enter(of_route(d.shape));
    (d, scope)
}

/// The level the turn of the agent named `agent` asks for: the agent's
/// own, else the route's, else `None`.
pub(crate) fn for_turn(agent: &str) -> Option<Effort> {
    for_turn_at(&super::specialists::base_dir(), agent)
}

/// [`for_turn`] against the store under `base`.
pub(crate) fn for_turn_at(base: &std::path::Path, agent: &str) -> Option<Effort> {
    let own = super::specialists::load_at(base)
        .into_iter()
        .find(|s| s.name == agent)
        .and_then(|s| s.effort.as_deref().and_then(Effort::parse));
    own.or_else(|| ROUTE.with(Cell::get))
}

#[cfg(test)]
#[path = "routeeffort_tests.rs"]
mod tests;
