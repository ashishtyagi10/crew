//! "What did I rule out?" and "forget correction 2" — answered from the
//! graph, like `recallask`, and for the same reason: memory is a thing crew
//! has, so it is read out and edited directly, not inferred by a model.
//!
//! WHY this door exists: a correction leads every recall it reaches and the
//! model is told it was "ruled out before". One caught by mistake, or one
//! that stopped being true, could not be seen or taken back (left open by
//! loop 30, S10). No new slash command: the question is the command.
use crate::PluginEvent;

use super::recall::{ago, now_ms};
use super::relay::smith_answer;
use super::session::Session;

/// What the message asks of the corrections.
#[derive(Debug, PartialEq)]
pub(crate) enum Ask {
    List,
    Forget(usize),
}

/// Whole-message phrases for the list; the forget form is these openings
/// and then a number. Exact, because forgetting writes: a sentence that
/// merely mentions a correction must fall through to the model.
const LISTS: &[&str] = &[
    "what did i rule out",
    "what have i ruled out",
    "what is ruled out",
    "what's ruled out",
    "what was ruled out",
    "show corrections",
    "list corrections",
];
const FORGETS: &[&str] = &["forget correction", "forget ruled out", "forget rule"];

pub(crate) fn asks(task: &str) -> Option<Ask> {
    let t = task.trim().trim_end_matches(['?', '.', '!']).to_lowercase();
    if LISTS.contains(&t.as_str()) {
        return Some(Ask::List);
    }
    let rest = FORGETS.iter().find_map(|f| t.strip_prefix(f))?;
    let n = rest.trim().trim_start_matches('#').parse().ok()?;
    Some(Ask::Forget(n))
}

/// The router's one call: list them, forget one, or nothing.
pub(crate) fn gate(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> Option<anyhow::Result<()>> {
    let ask = asks(task)?;
    Some(emit(smith_answer(answer(session, ask))))
}

pub(crate) fn answer(session: &Session, ask: Ask) -> String {
    let mut recall = super::recall::lock(&session.recall);
    match ask {
        Ask::List => list(&recall.ruled()),
        Ask::Forget(n) => match recall.forget_ruled(n) {
            Some(what) => format!("forgot correction {n}: {what}"),
            None => match recall.ruled().len() {
                0 => "nothing is ruled out in this project".to_string(),
                kept => format!("there is no correction {n} \u{2014} {kept} kept"),
            },
        },
    }
}

fn list(ruled: &[(String, u64)]) -> String {
    if ruled.is_empty() {
        return "nothing is ruled out in this project".to_string();
    }
    let now = now_ms();
    let mut out = vec!["ruled out in this project, newest first:".to_string()];
    out.extend(
        ruled
            .iter()
            .enumerate()
            .map(|(i, (what, ms))| format!("  {}. ({}) {what}", i + 1, ago(*ms, now))),
    );
    out.push("  \u{201c}forget correction 1\u{201d} drops one".to_string());
    out.join("\n")
}

#[cfg(test)]
#[path = "ruledask_tests.rs"]
mod tests;
