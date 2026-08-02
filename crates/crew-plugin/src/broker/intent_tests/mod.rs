//! Intent-router tests, split along responsibility lines: `grammar` covers
//! the `SHAPE:` parser and the injected-classifier seam; `routing` proves
//! each shape reaches its own capability path, the fallbacks, and the
//! plain-language parity for retired commands. Shared fixtures live here.
use super::*;

mod grammar;
mod routing;

fn text_of(ev: &PluginEvent) -> &str {
    match ev {
        PluginEvent::Message { text, .. } => text,
        _ => "",
    }
}

fn any_text(evs: &[PluginEvent], needle: &str) -> bool {
    evs.iter().any(|e| text_of(e).contains(needle))
}

/// Run `dispatch` for `shape` on a fresh session, collecting every event.
fn dispatch_collect(shape: Shape, task: &str) -> (Vec<PluginEvent>, Session) {
    let mut session = Session::new();
    let mut evs = Vec::new();
    dispatch(
        shape,
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
