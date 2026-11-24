//! `/review` — AI code review of the working tree (à la Codex's `/review`):
//! the reviewer agent reads the diff `/commit` would describe (staged wins,
//! else everything the tree has changed) and reports severity-ordered findings in
//! the pane, at the level the ask names: quick, standard or deep
//! (`reviewlevel`). Read-only, and the reviewer's CLI runs in its read-only mode
//! (`handsoff`): unlike `/commit` there is nothing to apply, so the
//! construct carries no session state.
use crate::PluginEvent;

use super::relay::msg;
use super::session::{call_timeout, Session};
use super::stdio::roster;

/// The one-completion prompt for the reviewer. `callers` is the
/// [`super::reviewcallers::section`] block ("" when nothing calls what the
/// diff changed): the call sites a diff cannot show.
pub(crate) fn review_prompt(diff: &str, callers: &str) -> String {
    let mut p = format!(
        "You are a strict code reviewer. Review this diff for correctness \
         bugs, edge cases, and risky patterns. Report only: change no file, \
         since the author decides what to fix.\n\
         Report each finding as one bullet, ordered worst-first by severity — \
         `blocker` (would break users), then `warn` (likely trouble), then \
         `nit` (style) — in the shape `severity — file:line — what and why`.\n\
         End with one verdict line that counts the findings, e.g. \
         `verdict: 2 blocker, 1 warn`. Only when there are none at all, \
         the whole review is the one line `no findings` — never under a \
         finding — rather than inventing issues.\n\nDIFF:\n{diff}"
    );
    if !callers.is_empty() {
        p.push_str("\n\n");
        p.push_str(callers);
    }
    p
}

/// Review the current diff at the level `task` asks for
/// (`reviewlevel`) and stream the findings back.
pub(crate) fn review_cmd(
    session: &mut Session,
    task: &str,
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
    // Read-only: a reviewer that fixes what it was asked to judge takes the
    // decision from the person who asked (`handsoff`); one with no read-only
    // mode has whatever it edited put back and said (`reviewguard`).
    let level = super::reviewlevel::Level::of_ask(task);
    let ran = reg
        .get(&author)
        .map(|a| super::reviewrun::run(level, &dir, &diff, a, call_timeout()));
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })?;
    let Some(ran) = ran else {
        return emit(msg(
            "agent smith",
            "review stopped — the reviewer went missing",
        ));
    };
    for note in ran.notes {
        emit(msg("agent smith", note))?;
    }
    match ran.reply {
        Ok(Some(t)) => {
            emit(msg(&format!("{author} → user"), t.text))?;
            match super::reviewfindings::dropped_note(&t.dropped) {
                Some(note) => emit(msg("agent smith", note)),
                None => Ok(()),
            }
        }
        Ok(None) => emit(msg("agent smith", "the review came back empty — try again")),
        Err(e) => emit(msg("agent smith", format!("review failed: {e}"))),
    }
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
