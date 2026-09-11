//! Intent-router tests, split along responsibility lines: `grammar` covers
//! the `SHAPE:`/`WHY:` parser and the injected-classifier seam; `routing`
//! proves each shape reaches its own capability path, the fallbacks, and the
//! plain-language parity for retired commands; `announce` proves the pane is
//! told the decision before the arm speaks; `hints` proves the model's
//! sizing lines reach the arms and the constants are backstops; `context`
//! proves the run says what it brings, after the routing line; `verify`
//! proves the `VERIFY:` line reaches the swarm and no other shape; `world`
//! proves the classifier is shown the room it routes in. Shared fixtures
//! live here.
use super::decision::{decide_in, parse_decision_on, Decision, Routing};
use super::*;
use crate::broker::testenv;

mod announce;
mod capability;
mod context;
mod grammar;
mod hints;
mod plangate;
mod routing;
mod verify;
mod world;

/// [`parse_decision_on`] with no roster — the bare grammar, where an
/// `AGENTS:` line can name nobody.
fn parse_decision(reply: &str) -> Option<Decision> {
    parse_decision_on(reply, &[])
}

/// [`decide_in`] with nothing known about the world.
fn decide(task: &str, classifier: Option<Classifier>) -> Routing {
    decide_in(task, &World::default(), classifier)
}

fn text_of(ev: &PluginEvent) -> &str {
    match ev {
        PluginEvent::Message { text, .. } => text,
        _ => "",
    }
}

fn any_text(evs: &[PluginEvent], needle: &str) -> bool {
    evs.iter().any(|e| text_of(e).contains(needle))
}

/// Run `dispatch` for `shape` with no sizing hints on a fresh session,
/// collecting every event.
fn dispatch_collect(shape: Shape, task: &str) -> (Vec<PluginEvent>, Session) {
    let mut session = Session::new();
    let mut evs = Vec::new();
    dispatch(
        shape,
        &Hints::default(),
        task,
        &mut session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    (evs, session)
}

/// Run the real `route` entry on a fresh session, collecting every event.
fn route_collect(task: &str) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut evs = Vec::new();
    route(
        task,
        &mut session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}

/// Run [`route_with`] under an injected classifier — the parity seam: a plain
/// phrasing goes in, and the events show which capability path answered.
fn route_stubbed(task: &str, call: Classifier) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut evs = Vec::new();
    route_with(
        task,
        Some(call),
        &mut session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}
