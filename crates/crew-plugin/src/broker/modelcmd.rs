//! `/model` — the whole model story in one construct: the grouped provider
//! picker ("your subscriptions" / "your keys" / "installed CLIs", each entry
//! numbered; signed-out delegated providers grayed with the exact sign-in
//! command), `/model <n>` to switch provider (persisted through the same
//! credential-store pin every model choice already uses), and the per-agent
//! model pinning that has always lived here. Moved out of `commands.rs` to
//! keep that file inside the line cap as the picker grew in.
use crate::PluginEvent;

use super::modelpick::{groups_text, select};
use super::relay::msg;
use super::session::Session;

/// `/model` — list providers + each agent's model; `/model <n>` — switch
/// provider; `/model <agent> <model>` — pin the agent to that model for
/// this session (`default` clears); `/model all <model|default>` — all at
/// once. Re-emits the roster so the pane's model badges update live.
pub(crate) fn model_cmd(
    session: &mut Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut parts = rest.split_whitespace();
    let (agent, model) = (parts.next(), parts.next());
    let Some(agent) = agent else {
        // The listing role `/agents` used to hold. It emitted a fresh Roster
        // first so the pane's badges picked up manifest edits without a
        // restart, and inheriting the report without inheriting that would
        // have quietly dropped a live-reload path.
        emit(PluginEvent::Roster {
            agents: session.registry().infos(),
        })?;
        let listing = format!(
            "{}\n\nagents:\n{}",
            groups_text(&super::auth::state::snapshot()),
            agents_report(session)
        );
        return emit(msg("agent smith", listing));
    };
    // `/model <n>` — pick a provider by its number in the grouped listing.
    if let Ok(n) = agent.parse::<usize>() {
        let note = select(&super::auth::state::snapshot(), n);
        emit(PluginEvent::Roster {
            agents: session.registry().infos(),
        })?;
        return emit(msg("agent smith", note));
    }
    // `/model all <model|default>` — apply one model across the whole roster
    // in a single step (what the pane's `/model` picker forwards).
    if agent.eq_ignore_ascii_case("all") {
        let Some(model) = model else {
            return emit(msg("agent smith", "usage: /model all <model|default>"));
        };
        let names = session.registry().names();
        if names.is_empty() {
            return emit(msg("agent smith", "no agents to configure"));
        }
        let note = if model.eq_ignore_ascii_case("default") {
            for n in &names {
                session.overrides.remove(n);
            }
            "all agents back on the provider default model".to_string()
        } else {
            for n in &names {
                session.overrides.insert(n.clone(), model.to_string());
            }
            match super::discover::pin_provider_for_model(model) {
                Some(p) => format!("all agents now run {model} (switched to {p})"),
                None => format!("all agents now run {model}"),
            }
        };
        emit(PluginEvent::Roster {
            agents: session.registry().infos(),
        })?;
        return emit(msg("agent smith", note));
    }
    let reg = session.registry();
    let Some(name) = reg
        .names()
        .into_iter()
        .find(|n| n.eq_ignore_ascii_case(agent))
    else {
        return emit(msg(
            "agent smith",
            format!(
                "unknown agent \u{201c}{agent}\u{201d} — agents: {}",
                reg.names().join(", ")
            ),
        ));
    };
    let Some(model) = model else {
        return emit(msg(
            "agent smith",
            format!("usage: /model {name} <model|default>"),
        ));
    };
    let note = if model.eq_ignore_ascii_case("default") {
        session.overrides.remove(&name);
        format!("{name} back on the provider default model")
    } else {
        session.overrides.insert(name.clone(), model.to_string());
        format!("{name} now runs {model}")
    };
    emit(PluginEvent::Roster {
        agents: session.registry().infos(),
    })?;
    emit(msg("agent smith", note))
}

/// The roster, one agent per line: name, role hint, and the model it runs.
fn agents_report(session: &Session) -> String {
    let reg = session.registry();
    if reg.is_empty() {
        return super::stdio::roster(&reg);
    }
    let lines: Vec<String> = reg
        .infos()
        .iter()
        .map(|a| {
            let model = if a.model.is_empty() {
                "(own model)".to_string()
            } else {
                a.model.clone()
            };
            format!("\u{25aa} {} \u{2014} {} \u{2014} {model}", a.name, a.role)
        })
        .collect();
    lines.join("\n")
}
