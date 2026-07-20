//! Plan mode (à la Claude Code): `/plan <task>` has an agent draft a numbered
//! plan without executing anything; the draft then waits until the user runs
//! `/approve` (the crew executes it) or `/reject` (it is discarded). The
//! pending plan is shared session state, so a draft made on the worker thread
//! is visible to the inline `/reject` and the next `/approve`.
use std::sync::{Arc, Mutex, MutexGuard};

use crate::PluginEvent;

use super::relay::{msg, relay_turn, split_target};
use super::route::clip;
use super::session::{call_timeout, Session};
use super::stdio::roster;

/// A drafted plan awaiting the user's verdict.
pub(crate) struct PendingPlan {
    pub task: String,
    pub plan: String,
    pub author: String,
}

/// The session's pending plan, shared between the stdin loop and the worker.
pub(crate) type SharedPlan = Arc<Mutex<Option<PendingPlan>>>;

fn lock(plan: &SharedPlan) -> MutexGuard<'_, Option<PendingPlan>> {
    plan.lock().unwrap_or_else(|e| e.into_inner())
}

/// `/plan <task>`: an agent (`@agent` selects who) drafts a numbered plan —
/// steps only, no execution — and the session holds it for `/approve`.
pub(crate) fn plan_cmd(
    session: &mut Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let task = rest.trim();
    if task.is_empty() {
        return emit(msg(
            "agent smith",
            "usage: /plan <task> \u{2014} draft first, run on /approve",
        ));
    }
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("agent smith", roster(&reg)));
    }
    let (author, task) = split_target(task, &reg);
    emit(msg(
        "agent smith",
        format!("plan mode \u{2014} {author} drafts; nothing runs until /approve"),
    ))?;
    emit(PluginEvent::Activity {
        agent: author.clone(),
        state: "thinking".into(),
        from: "plan".into(),
    })?;
    let reply = reg
        .get(&author)
        .map(|a| a.call(&plan_prompt(&task), call_timeout()));
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })?;
    let plan = match reply {
        Some(Ok(r)) => strip_control(&r),
        Some(Err(e)) => return emit(msg("agent smith", format!("plan draft failed: {e}"))),
        None => {
            return emit(msg(
                "agent smith",
                "plan stopped \u{2014} the drafting agent went missing",
            ))
        }
    };
    if plan.is_empty() {
        return emit(msg(
            "agent smith",
            "plan draft came back empty \u{2014} try again",
        ));
    }
    emit(msg(&format!("{author} \u{2192} user"), plan.clone()))?;
    *lock(&session.plan) = Some(PendingPlan { task, plan, author });
    emit(msg(
        "agent smith",
        "plan ready \u{2014} /approve runs it, /reject discards it",
    ))
}

/// `/approve`: execute the pending plan as a relay turn led by its author.
pub(crate) fn approve_cmd(
    session: &mut Session,
    tick_emit: &std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let Some(p) = lock(&session.plan).take() else {
        return emit(msg(
            "agent smith",
            "no plan pending \u{2014} draft one with /plan <task>",
        ));
    };
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("agent smith", roster(&reg)));
    }
    // The author leads execution while it is still on the roster.
    let start = if reg.get(&p.author).is_some() {
        p.author.clone()
    } else {
        reg.names().into_iter().next().unwrap_or_default()
    };
    emit(msg(
        "agent smith",
        format!("plan approved \u{2014} {start} leads execution"),
    ))?;
    let broker = session.broker(reg);
    relay_turn(
        &broker,
        &start,
        &execute_body(&p.task, &p.plan),
        "plan",
        tick_emit,
        emit,
    )?;
    Ok(())
}

/// `/reject`: drop the pending plan without running it.
pub(crate) fn reject_cmd(
    session: &mut Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let had = lock(&session.plan).take().is_some();
    emit(msg(
        "agent smith",
        if had {
            "plan discarded"
        } else {
            "no plan pending \u{2014} draft one with /plan <task>"
        },
    ))
}

/// The drafting instruction: a plan, not an execution.
pub(crate) fn plan_prompt(task: &str) -> String {
    format!(
        "You are in plan mode. Draft a concise numbered plan for the task below \
         \u{2014} the steps, the files or components involved, and the main risks. \
         Do NOT execute anything or produce final code yet.\n\nTask: {task}\n\n\
         Reply with the plan only."
    )
}

/// The execution instruction handed to the relay after `/approve`.
pub(crate) fn execute_body(task: &str, plan: &str) -> String {
    format!(
        "Execute this approved plan.\n\nTask: {task}\n\nApproved plan:\n{}\n\n\
         Follow the steps in order; call out any deviation from the plan.",
        clip(plan, 1500)
    )
}

/// Strip a trailing routing directive (`@done` / `@next \u{2026}`) an agent may
/// append out of habit — the draft is user-facing, not routed.
pub(crate) fn strip_control(reply: &str) -> String {
    match crate::parse_routing(reply) {
        crate::Routing::Done(b) | crate::Routing::Relay { body: b, .. } if !b.is_empty() => b,
        _ => reply.trim().to_string(),
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
