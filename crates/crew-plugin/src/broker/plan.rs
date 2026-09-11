//! Plan mode (à la Claude Code): a plan-shaped ask has an agent draft a
//! numbered plan without executing anything; the draft then waits until the
//! user says "approve" or "reject" — the conversational gate in
//! `intent::gate`, or the pane's enter/esc. An approved plan runs as the
//! SWARM, with its steps as the task breakdown (`approved_goal`): the plan the
//! user read is the plan that runs, task for step, not a relay's paraphrase
//! of it. The pending plan is shared session state, so a draft made on the
//! worker thread is visible to a verdict arriving on another send.
use std::sync::{Arc, Mutex, MutexGuard};

use crate::PluginEvent;

use super::relay::{msg, split_target};
use super::session::{call_timeout, Session};
use super::stdio::roster;

/// Chars of the plan carried into the approved goal — a plan is short by
/// construction (steps, not code), so this only bounds a runaway draft.
const PLAN_CAP: usize = 6_000;

/// A drafted plan awaiting the user's verdict.
pub(crate) struct PendingPlan {
    pub task: String,
    pub plan: String,
    /// The router's `VERIFY: yes` at draft time: the run that follows the
    /// approval is judged against the request when it ends (`swarmverify`).
    pub verify: bool,
}

/// The session's pending plan, shared between the stdin loop and the worker.
pub(crate) type SharedPlan = Arc<Mutex<Option<PendingPlan>>>;

fn lock(plan: &SharedPlan) -> MutexGuard<'_, Option<PendingPlan>> {
    plan.lock().unwrap_or_else(|e| e.into_inner())
}

/// The plan shape: an agent (`@agent` selects who) drafts a numbered plan —
/// steps only, no execution — and the session holds it for the verdict.
/// `verify` is the router's `VERIFY: yes`, kept with the draft for the run.
pub(crate) fn plan_cmd(
    session: &mut Session,
    rest: &str,
    verify: bool,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let task = rest.trim();
    if task.is_empty() {
        return emit(msg(
            "agent smith",
            "nothing to plan \u{2014} say what the plan is for",
        ));
    }
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("agent smith", roster(&reg)));
    }
    let (author, task) = split_target(task, &reg);
    emit(msg(
        "agent smith",
        format!("plan mode \u{2014} {author} drafts; nothing runs until you approve"),
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
    *lock(&session.plan) = Some(PendingPlan { task, plan, verify });
    // The host turns this into the decision affordance. The message stays for
    // hosts that render text only (the broker is driven over stdio by more
    // than the crew pane), but it no longer has to teach two constructs.
    emit(PluginEvent::Plan { pending: true })?;
    emit(msg(
        "agent smith",
        // Both forms: the crew pane binds the keys, but the broker is driven
        // over stdio by hosts that have no keyboard at all, and telling those
        // to "press enter" is telling them nothing.
        "plan ready \u{2014} enter runs it, esc discards it (or say \
         \u{201c}approve\u{201d} / \u{201c}reject\u{201d})",
    ))
}

/// "approve": run the pending plan as the swarm. The approved steps head the
/// goal (`approved_goal`) and the planner is told to mirror them one-to-one
/// (`PLANNER_SYSTEM`), so the graph the crew runs IS the plan the user read.
/// Keyless and mock runs take the same path: their stub planner ignores the
/// text, but the shape of the run — plan, cast, tasks — is the one shape.
pub(crate) fn approve_cmd(
    session: &mut Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let Some(p) = lock(&session.plan).take() else {
        return emit(msg(
            "agent smith",
            "no plan pending \u{2014} ask for one first (\u{201c}draft a plan for \u{2026}\u{201d})",
        ));
    };
    emit(PluginEvent::Plan { pending: false })?;
    emit(msg("agent smith", "running the approved plan as a swarm"))?;
    super::swarm::run_task(&approved_goal(&p.task, &p.plan), p.verify, session, emit)
}

/// "reject": drop the pending plan without running it.
pub(crate) fn reject_cmd(
    session: &mut Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let had = lock(&session.plan).take().is_some();
    emit(PluginEvent::Plan { pending: false })?;
    emit(msg(
        "agent smith",
        if had {
            "plan discarded"
        } else {
            "no plan pending \u{2014} ask for one first (\u{201c}draft a plan for \u{2026}\u{201d})"
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

/// The swarm's goal after an approval: the plan first, under the header the
/// planner's prompt names, then the request it was drafted for. The plan's
/// lines are kept — `route::clip` would fold its steps into one line, and a
/// numbered list is what the planner mirrors — so the bound is a byte cut.
pub(crate) fn approved_goal(task: &str, plan: &str) -> String {
    let mut cut = plan.len().min(PLAN_CAP);
    while !plan.is_char_boundary(cut) {
        cut -= 1;
    }
    format!(
        "APPROVED PLAN \u{2014} follow these steps as the task breakdown; keep their \
         order and dependencies:\n{}\n\nGOAL:\n{task}",
        &plan[..cut]
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
