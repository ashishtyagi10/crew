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
            (None, Some(NoAnswer::Stalled)) => {
                out.push_str(&format!("[{who}] no answer (stalled)\n"))
            }
            (None, _) => out.push_str(&format!("[{who}] no answer (idle)\n")),
        }
    }
    (out.trim_end().to_string(), if answered > 0 { 0 } else { 2 })
}

/// Render the `crew panes` roster as a fixed-column table.
pub(crate) fn render_roster(panes: &[PaneCard]) -> String {
    let mut out = String::from("id   label            kind      running   state\n");
    for c in panes {
        out.push_str(&format!(
            "{:<4} {:<16} {:<9} {:<9} {}\n",
            c.id,
            c.label.as_deref().unwrap_or("-"),
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
