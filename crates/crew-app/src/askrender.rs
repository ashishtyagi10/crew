//! Client-side rendering for the `ask`/`panes`/broadcast replies: turns an
//! [`crate::ipc_types::Reply`] into the human text + process exit code the
//! `crew` CLI prints. Pure formatting, split out of [`crate::askclient`] so the
//! command/transport code stays small.
use crate::ipc_types::{CastAnswer, NoAnswer, PaneCard, Reply};

/// Render a reply to (human text, process exit code): 0 answered/roster,
/// 2 no-answer, 3 unreachable/no-crew.
pub(crate) fn render(r: &Reply) -> (String, i32) {
    match r {
        Reply::Answered { text } => (format!("ANSWERED: {text}"), 0),
        Reply::NoAnswer { reason, partial } => {
            let why = match reason {
                NoAnswer::IdleNoEngage => "idle (target never engaged)",
                NoAnswer::Stalled => "stalled (target stopped without finishing)",
                NoAnswer::BusyElsewhere => "busy (target is working on its own task)",
                NoAnswer::Unreachable => "unreachable (no such pane)",
            };
            let mut s = format!("NO_ANSWER: {why}");
            if let Some(p) = partial.as_deref().filter(|p| !p.is_empty()) {
                s.push_str(&format!("\n--- partial ---\n{p}"));
            }
            let code = if matches!(reason, NoAnswer::Unreachable) {
                3
            } else {
                2
            };
            (s, code)
        }
        Reply::Roster { panes } => (render_roster(panes), 0),
        Reply::Cast { answers } => render_cast(answers),
        // Only reachable if an ask was pointed at the daemon endpoint. Say so
        // rather than rendering a status line the caller never asked for.
        // A failure the far side chose to explain — surface its words, not ours.
        Reply::Failed { message } => (format!("FAILED: {message}"), 2),
        Reply::Daemon { .. }
        | Reply::Session { .. }
        | Reply::Sessions { .. }
        | Reply::Closed { .. }
        | Reply::Sent { .. }
        | Reply::Events { .. }
        | Reply::Channels { .. }
        | Reply::Pressed { .. }
        | Reply::Watched { .. }
        | Reply::Watchlist { .. }
        | Reply::Unwatched { .. }
        | Reply::Snoozed { .. } => (
            "NO_ANSWER: unreachable (that endpoint is the crew daemon, not a pane)".to_string(),
            3,
        ),
    }
}

/// Render a broadcast reply to (text, exit code): 0 if anyone answered, 2 if
/// panes were reached but none answered, 3 if no pane was eligible.
fn render_cast(answers: &[CastAnswer]) -> (String, i32) {
    if answers.is_empty() {
        return (
            "NO_ANSWER: no eligible panes to broadcast to".to_string(),
            3,
        );
    }
    let mut out = String::new();
    let mut answered = 0;
    for a in answers {
        let who = a.label.as_deref().unwrap_or(&a.pane);
        match (&a.text, a.no_answer) {
            (Some(t), _) => {
                answered += 1;
                out.push_str(&format!("[{who}] ANSWERED: {t}\n"));
            }
            (None, why) => out.push_str(&format!("[{who}] no answer ({})\n", word(why))),
        }
    }
    (out.trim_end().to_string(), if answered > 0 { 0 } else { 2 })
}

/// The one word for why a pane did not answer — the same four `render`
/// spells out, so a broadcast never calls a busy pane idle.
fn word(reason: Option<NoAnswer>) -> &'static str {
    match reason {
        Some(NoAnswer::Stalled) => "stalled",
        Some(NoAnswer::BusyElsewhere) => "busy",
        Some(NoAnswer::Unreachable) => "unreachable",
        Some(NoAnswer::IdleNoEngage) | None => "idle",
    }
}

/// Render the `crew panes` roster as a fixed-column table — or say there is
/// none, since a bare header row reads like a broken query.
pub(crate) fn render_roster(panes: &[PaneCard]) -> String {
    if panes.is_empty() {
        return "no panes open".to_string();
    }
    let mut out = String::from("id   label            kind      running   state\n");
    for c in panes {
        out.push_str(&format!(
            "{:<4} {:<16} {:<9} {:<9} {}\n",
            c.id,
            crate::chatwidth::clip_w(c.label.as_deref().unwrap_or("-"), 16),
            c.kind,
            c.running.as_deref().unwrap_or("-"),
            if c.busy { "busy" } else { "idle" },
        ));
    }
    out.trim_end().to_string()
}

#[cfg(test)]
#[path = "askrender_tests.rs"]
mod tests;
