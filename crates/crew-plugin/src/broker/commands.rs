//! Slash constructs typed into the `/crew` pane. Anything starting with `/`
//! is a broker command rather than a task: `/help` lists the constructs,
//! `/agents` reports the roster with each agent's current model.
use crate::PluginEvent;

use super::relay::msg;
use super::session::Session;

/// Whether `text` addresses the broker's command router.
pub(crate) fn is_command(text: &str) -> bool {
    text.trim_start().starts_with('/')
}

/// Whether a command answers instantly (no agent calls) — those run inline on
/// the stdin loop even while a long construct occupies the worker thread.
pub(crate) fn is_quick(text: &str) -> bool {
    let line = text.trim().trim_start_matches('/');
    let cmd = line.split_whitespace().next().unwrap_or("");
    is_command(text) && !matches!(cmd, "fan" | "loop" | "goal" | "skill")
}

/// One-line summaries of every construct, shown by `/help`.
pub(crate) const HELP: &str = "constructs:\n\
    /help — this list\n\
    /agents — the roster with each agent's model\n\
    /model <agent> <model|default> — pin an agent to a model (mix models freely)\n\
    /fan <task> — every agent answers the same task in parallel\n\
    /loop <n> <task> — n relay rounds, each improving the last answer\n\
    /goal <text> — keep working until a judge agent rules the goal met\n\
    /skills — list prompt playbooks (~/.config/crew/skills, .crew/skills)\n\
    /skill <name> <task> — run the relay with that playbook prepended\n\
    /stop — cancel the running construct at the next checkpoint\n\
    /status — session totals, models, and what's running\n\
    @<agent> <task> — choose who starts the relay\n\
    @<a>+<b> <task> — those agents answer in parallel";

/// Handle a `/command` line; emits reply events through `emit`.
pub(crate) fn handle(
    session: &mut Session,
    text: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let line = text.trim().trim_start_matches('/');
    let (cmd, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    match cmd {
        "help" => emit(msg("crew", HELP)),
        "agents" => emit(msg("crew", agents_report(session))),
        "model" => model_cmd(session, rest, emit),
        "fan" => fan_cmd(session, rest, emit),
        "loop" => super::constructs::loop_cmd(session, rest, emit),
        "goal" => super::constructs::goal_cmd(session, rest, emit),
        "skills" => emit(msg(
            "crew",
            super::skills::list_report(&super::skills::load()),
        )),
        "skill" => super::skills::skill_cmd(session, rest, emit),
        "status" => emit(msg("crew", status_report(session))),
        other => emit(msg(
            "crew",
            format!("unknown construct /{other} — try /help"),
        )),
    }
}

/// `/model` — list each agent's model; `/model <agent> <model>` — pin the
/// agent to that model for this session; `default` clears the pin. Re-emits
/// the roster so the pane's model badges update live.
fn model_cmd(
    session: &mut Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut parts = rest.split_whitespace();
    let (agent, model) = (parts.next(), parts.next());
    let Some(agent) = agent else {
        return emit(msg("crew", agents_report(session)));
    };
    let reg = session.registry();
    let Some(name) = reg
        .names()
        .into_iter()
        .find(|n| n.eq_ignore_ascii_case(agent))
    else {
        return emit(msg(
            "crew",
            format!(
                "unknown agent \u{201c}{agent}\u{201d} — agents: {}",
                reg.names().join(", ")
            ),
        ));
    };
    let Some(model) = model else {
        return emit(msg("crew", format!("usage: /model {name} <model|default>")));
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
    emit(msg("crew", note))
}

/// `/fan <task>` — every agent answers `task` concurrently.
fn fan_cmd(
    session: &mut Session,
    task: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let task = task.trim();
    if task.is_empty() {
        return emit(msg("crew", "usage: /fan <task>"));
    }
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("crew", super::stdio::roster(&reg)));
    }
    let names = reg.names();
    emit(msg(
        "crew",
        format!("fanning out to {} agents in parallel\u{2026}", names.len()),
    ))?;
    super::fan::fan_out(&reg, &names, task, super::session::call_timeout(), emit)
}

/// `/status` — what the session has done and is doing right now.
fn status_report(session: &Session) -> String {
    use std::sync::atomic::Ordering;
    let running = session
        .running()
        .map(|l| format!("running \u{2018}{l}\u{2019}"))
        .unwrap_or_else(|| "idle".into());
    let pins = if session.overrides.is_empty() {
        "none".to_string()
    } else {
        let mut pins: Vec<String> = session
            .overrides
            .iter()
            .map(|(a, m)| format!("{a} \u{2192} {m}"))
            .collect();
        pins.sort();
        pins.join(", ")
    };
    format!(
        "status: {running}\n\
         turns: {} \u{00b7} ~{} tok (approx)\n\
         model pins: {pins}\n\n{}",
        session.turns.load(Ordering::Relaxed),
        session.tokens.load(Ordering::Relaxed),
        agents_report(session),
    )
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

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
