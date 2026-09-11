//! The fan capability's entry: every agent answers the same task in
//! parallel. Lived in `commands.rs` while `/fan` was a command; the command
//! is retired, the intent router is the only caller, so the body moved here
//! (the concurrency itself stays in `broker::fan::fan_out`).
use std::sync::Arc;

use crate::PluginEvent;

use crate::broker::relay::msg;
use crate::broker::session::Session;

/// Fan `task` out to `only` — the model's chosen subset, already filtered
/// to roster names — or to the whole roster, streaming replies fastest-first.
pub(super) fn fan_cmd(
    session: &mut Session,
    task: &str,
    only: Option<&[String]>,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let task = task.trim();
    if task.is_empty() {
        return emit(msg("agent smith", "usage: /fan <task>"));
    }
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("agent smith", crate::broker::stdio::roster(&reg)));
    }
    let names = match only {
        Some(subset) if !subset.is_empty() => subset.to_vec(),
        _ => reg.names(),
    };
    let to = if only.is_some_and(|s| !s.is_empty()) {
        names.join(", ")
    } else {
        format!("{} agents", names.len())
    };
    emit(msg(
        "agent smith",
        format!("fanning out to {to} in parallel\u{2026}"),
    ))?;
    fan_recorded(session, &reg, &names, task, tick_emit, emit)
}

/// [`fan_out`](crate::broker::fan::fan_out) as one turn of the pane's thread:
/// the recent turns ride in front of `task` (standing memory folds in inside
/// `fan_out`, on top), and the replies that landed are recorded as the turn's
/// combined answer — none landed, or the user stopped it, and nothing is.
/// Shared with the `@a+b` dial in `stdio`, which is the same fan by hand.
pub(crate) fn fan_recorded(
    session: &Session,
    reg: &crate::Registry,
    names: &[String],
    task: &str,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let body = crate::broker::thread::with_context(&session.thread, task);
    let timeout = crate::broker::session::call_timeout();
    let replies = crate::broker::fan::fan_out(reg, names, &body, timeout, tick_emit, emit)?;
    let answer = crate::broker::thread::combined(replies).filter(|_| !session.cancelled());
    crate::broker::thread::record(&session.thread, task, answer);
    Ok(())
}
