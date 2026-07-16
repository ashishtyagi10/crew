//! `/review` — AI code review of the working tree (à la Codex's `/review`):
//! the reviewer agent reads the diff `/commit` would describe (staged wins,
//! else unstaged tracked changes) and reports severity-ordered findings in
//! the pane. Read-only: unlike `/commit` there is nothing to apply, so the
//! construct carries no session state.
use crate::PluginEvent;

use super::constructs::{is_critic, pick_by_role};
use super::relay::msg;
use super::session::{call_timeout, Session};
use super::stdio::roster;

/// The one-completion prompt for the reviewer.
pub(crate) fn review_prompt(diff: &str) -> String {
    format!(
        "You are a strict code reviewer. Review this diff for correctness \
         bugs, edge cases, and risky patterns.\n\
         Report each finding as one bullet, ordered worst-first by severity — \
         `blocker` (would break users), then `warn` (likely trouble), then \
         `nit` (style) — in the shape `severity — file:line — what and why`.\n\
         End with a one-line verdict. If the diff is clean, say so in one \
         line (\"no findings\") instead of inventing issues.\n\nDIFF:\n{diff}"
    )
}

/// `/review`: review the current diff and stream the findings back.
pub(crate) fn review_cmd(
    session: &mut Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("crew", format!("review: no working directory: {e}"))),
    };
    let (diff, staged) = match super::gitmsg::pick_diff(&dir) {
        Err(e) => return emit(msg("crew", format!("review: {e}"))),
        Ok(None) => {
            return emit(msg(
                "crew",
                "nothing to review — the tree is clean (stage or edit something)",
            ))
        }
        Ok(Some(d)) => d,
    };
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("crew", roster(&reg)));
    }
    // Elected by the agent's OWN role (`is_critic`), not the literal name
    // "reviewer" — no invented specialist is ever literally called that, so
    // this used to always fall through to the roster's first (arbitrary,
    // LRU-ordered) agent. See `constructs::pick_judge`, which solves the
    // identical problem for `/goal`.
    let author = pick_by_role(&reg.infos(), is_critic);
    emit(msg(
        "crew",
        format!(
            "reviewing the {} diff…",
            if staged { "staged" } else { "unstaged" }
        ),
    ))?;
    emit(PluginEvent::Activity {
        agent: author.clone(),
        state: "thinking".into(),
        from: "review".into(),
    })?;
    let reply = reg
        .get(&author)
        .map(|a| a.call(&review_prompt(&diff), call_timeout()));
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })?;
    match reply {
        Some(Ok(r)) if !r.trim().is_empty() => emit(msg(&format!("{author} → user"), r)),
        Some(Ok(_)) => emit(msg("crew", "the review came back empty — try again")),
        Some(Err(e)) => emit(msg("crew", format!("review failed: {e}"))),
        None => emit(msg("crew", "review stopped — the reviewer went missing")),
    }
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
