//! `/review` — AI code review of the working tree (à la Codex's `/review`):
//! the reviewer agent reads the diff `/commit` would describe (staged wins,
//! else everything the tree has changed) and reports severity-ordered findings in
//! the pane. Read-only: unlike `/commit` there is nothing to apply, so the
//! construct carries no session state.
use crate::PluginEvent;

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
    let dir = match super::gitmsg::project_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("agent smith", format!("review: {e}"))),
    };
    let (diff, staged) = match super::gitmsg::pick_diff(&dir) {
        Err(e) => return emit(msg("agent smith", format!("review: {e}"))),
        Ok(None) => {
            return emit(msg(
                "agent smith",
                "nothing to review — the tree is clean (stage or edit something)",
            ))
        }
        Ok(Some(d)) => d,
    };
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("agent smith", roster(&reg)));
    }
    // The MODEL elects the reviewer from the live roster (`elect`); keyless
    // and mock runs get the deterministic roster-first fallback.
    let author = super::elect::elect(
        "review a code diff and report findings worst-first",
        &reg.infos(),
        None,
    );
    emit(msg(
        "agent smith",
        format!(
            "reviewing the {} diff…",
            if staged { "staged" } else { "working" }
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
        Some(Ok(_)) => emit(msg("agent smith", "the review came back empty — try again")),
        Some(Err(e)) => emit(msg("agent smith", format!("review failed: {e}"))),
        None => emit(msg(
            "agent smith",
            "review stopped — the reviewer went missing",
        )),
    }
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
