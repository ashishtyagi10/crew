//! Where the tools note lands on the native path.
//!
//! The native path sends the tool schemas on the wire and no prose about them
//! — until this file, not one word. That was right while every tool fit: a
//! model shown the whole catalog needs no footnote. It is wrong the moment the
//! picker (`Tools::specs_for`) leaves tools out, because the model is then
//! shown a partial list and has no way to know it is partial, nor that one of
//! the tools it WAS shown searches the rest. [`Tools::note_for`] is that one
//! sentence; this decides where it goes.
//!
//! It rides the system prompt when the task has one, and the user turn
//! otherwise — never a system prompt of its own. A system prompt that is
//! nothing but a footnote about tools would make the footnote the agent's
//! whole persona.
//!
//! [`Tools::note_for`]: crate::tools::Tools::note_for

#[cfg(test)]
#[path = "note_tests.rs"]
mod tests;

/// `(system, prompt)` with `note` appended to whichever the model reads first.
///
/// `None` for the note leaves BOTH byte-identical: a prompt with every tool on
/// the wire must be the prompt it was before retrieval existed, or a cached
/// provider response stops being cached.
pub(super) fn noted(
    system: Option<String>,
    prompt: String,
    note: Option<String>,
) -> (Option<String>, String) {
    let Some(note) = note else {
        return (system, prompt);
    };
    match system {
        Some(s) => (Some(format!("{s}\n\n{note}")), prompt),
        None => (None, format!("{prompt}\n\n{note}")),
    }
}
