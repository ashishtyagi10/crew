//! `/standup [days]` — an AI standup update from the repo's recent commits:
//! what shipped (grouped by theme), what looks in progress, and any risks,
//! written in the first person so it can be pasted straight into the morning
//! thread. History summarization — the complement of `/review` (the diff you
//! haven't committed) and `/commit` (the message for it).
use std::path::Path;

use crate::PluginEvent;

use super::constructs::{is_writer, pick_by_role};
use super::relay::msg;
use super::session::{call_timeout, Session};
use super::stdio::roster;

/// Log budget interpolated into the prompt.
const LOG_CAP: usize = 10_000;
/// Widest lookback `/standup <days>` accepts.
const MAX_DAYS: u32 = 30;

/// Parse the optional `[days]` argument: empty → 1; a number is clamped to
/// `1..=MAX_DAYS`; anything else → `None` (usage).
pub(crate) fn parse_days(rest: &str) -> Option<u32> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Some(1);
    }
    rest.parse::<u32>().ok().map(|d| d.clamp(1, MAX_DAYS))
}

/// The repo's commits from the last `days` days, oldest first — `Ok(None)`
/// when there are none in the window, `Err` outside a repository. Clipped to
/// [`LOG_CAP`].
pub(crate) fn recent_log(dir: &Path, days: u32) -> Result<Option<String>, String> {
    let out = std::process::Command::new("git")
        .current_dir(dir)
        .args([
            "log",
            &format!("--since={days} days ago"),
            "--reverse",
            "--pretty=format:%h %s (%an, %ar)",
        ])
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        // A repo with no commits yet isn't an error — there's just nothing
        // to report.
        if err.contains("does not have any commits yet") {
            return Ok(None);
        }
        return Err(err);
    }
    let mut log = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if log.is_empty() {
        return Ok(None);
    }
    if log.len() > LOG_CAP {
        let mut cut = LOG_CAP;
        while !log.is_char_boundary(cut) {
            cut -= 1;
        }
        log.truncate(cut);
        log.push_str("\n… (log clipped)");
    }
    Ok(Some(log))
}

/// The one-completion prompt.
pub(crate) fn standup_prompt(log: &str, days: u32) -> String {
    format!(
        "Write a concise STANDUP update in the first person from the last \
         {days} day(s) of commits below.\n\
         Shape: `Done:` bullets grouped by theme (not one per commit), then \
         `In progress:` for work the commits imply is unfinished, then \
         `Risks/blockers:` — or `none` when there are none. No preamble.\n\n\
         COMMITS (oldest first):\n{log}"
    )
}

/// `/standup [days]`: summarize the window's commits as a standup update.
pub(crate) fn standup_cmd(
    session: &mut Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let Some(days) = parse_days(rest) else {
        return emit(msg(
            "crew",
            "usage: /standup [days] — summarize recent commits",
        ));
    };
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("crew", format!("standup: no working directory: {e}"))),
    };
    let log = match recent_log(&dir, days) {
        Err(e) => return emit(msg("crew", format!("standup: {e}"))),
        Ok(None) => {
            return emit(msg(
                "crew",
                format!("no commits in the last {days} day(s) — nothing to report"),
            ))
        }
        Ok(Some(l)) => l,
    };
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("crew", roster(&reg)));
    }
    // Elected by the agent's OWN role (`is_writer`), not the literal name
    // "coder" — see `review.rs`'s identical fix and `constructs::pick_judge`.
    let author = pick_by_role(&reg.infos(), is_writer);
    emit(msg(
        "crew",
        format!("drafting a standup from the last {days} day(s) of commits…"),
    ))?;
    emit(PluginEvent::Activity {
        agent: author.clone(),
        state: "thinking".into(),
        from: "standup".into(),
    })?;
    let reply = reg
        .get(&author)
        .map(|a| a.call(&standup_prompt(&log, days), call_timeout()));
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })?;
    match reply {
        Some(Ok(r)) if !r.trim().is_empty() => emit(msg(&format!("{author} → user"), r)),
        Some(Ok(_)) => emit(msg("crew", "the standup came back empty — try again")),
        Some(Err(e)) => emit(msg("crew", format!("standup failed: {e}"))),
        None => emit(msg("crew", "standup stopped — the coder went missing")),
    }
}

#[cfg(test)]
#[path = "standup_tests.rs"]
mod tests;
