//! "What do you remember about the router?" — answered from the graph, not
//! by a model.
//!
//! The recall graph (`super::recall`) rides in front of every task, so a
//! model COULD answer this; it would cost a call, it would paraphrase, and
//! with no provider signed in it would not answer at all. Memory is a thing
//! crew has, not a thing it infers, and a question about it should be
//! answered by reading it out: the standing notes, the project's instruction
//! files, the graph's size, and — when the question named a subject — the
//! turns and files that subject actually reaches.
//!
//! The phrases are matched by PREFIX rather than exactly (unlike the switches
//! in `undoask` and `planfirst`), because the interesting part is what comes
//! after them. Nothing here writes anything, so a false positive costs a
//! wrong answer and never a wrong action.
use crate::PluginEvent;

use super::relay::msg;
use super::session::Session;

/// Chars of the standing notes read back.
const NOTES_CAP: usize = 600;

/// The subject the question asked about — `Some("")` when it asked about
/// everything ("what do you remember?").
pub(crate) fn asks(task: &str) -> Option<String> {
    let t = task.trim().trim_end_matches(['?', '.', '!']).to_lowercase();
    const LEADS: &[&str] = &[
        "what do you remember",
        "what do you know",
        "do you remember",
        "what have we said",
        "what did we say",
        "what do we know",
    ];
    let lead = LEADS.iter().find(|l| t.starts_with(**l))?;
    let rest = t[lead.len()..]
        .trim()
        .trim_start_matches("about")
        .trim_start_matches("of")
        .trim();
    Some(rest.to_string())
}

/// The router's one call: read the memory out, or nothing.
pub(crate) fn gate(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> Option<anyhow::Result<()>> {
    let subject = asks(task)?;
    Some(emit(msg("agent smith", answer(session, &subject))))
}

/// What crew has, read out.
pub(crate) fn answer(session: &Session, subject: &str) -> String {
    let mut out = vec![match subject.is_empty() {
        true => "what I remember in this project:".to_string(),
        false => format!("what I remember about \u{201c}{subject}\u{201d}:"),
    }];
    let recall = super::recall::lock(&session.recall);
    if !subject.is_empty() {
        match recall.context(subject, &[]) {
            Some(block) => out.extend(block.lines().skip(1).map(|l| format!("  {l}"))),
            None => out.push("  nothing in the recall graph reaches it yet".into()),
        }
    }
    let (turns, topics, files) = recall.stats();
    out.push(format!(
        "  graph: {turns} turn(s) \u{b7} {topics} topic(s) \u{b7} {files} file(s)"
    ));
    drop(recall);
    match super::memory::load() {
        Some(m) => {
            out.push("  standing notes (.crew/memory.md):".into());
            out.extend(
                super::thread::clip_chars(&m, NOTES_CAP)
                    .lines()
                    .map(|l| format!("    {}", l.trim_start_matches("- "))),
            );
        }
        None => out.push("  no standing notes \u{2014} #<note> starts one".into()),
    }
    if let Some((_, names)) = super::agentsmd::block() {
        out.push(format!("  project instructions: {}", names.join(", ")));
    }
    out.join("\n")
}

/// The subject's turns, counted rather than rendered — the seam a test uses
/// to check the answer quoted what the graph actually holds.
#[cfg(test)]
pub(crate) fn turns_about(session: &Session, subject: &str) -> usize {
    super::recall::lock(&session.recall)
        .recalled(subject, &[])
        .map_or(0, |r| r.turns)
}

#[cfg(test)]
#[path = "recallask_tests.rs"]
mod tests;
