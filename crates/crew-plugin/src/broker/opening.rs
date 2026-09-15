//! Where the project stands, said once as a pane opens.
//!
//! A new pane used to open on a nameplate and a quote: true to the aesthetic,
//! and silent about the only thing you actually want to know when you sit
//! back down — what were we doing, and is it broken? Both answers are already
//! on disk. The recall graph holds the last turn crew took in this tree and
//! what the project's check last said about it; git holds whether the working
//! tree still has uncommitted work in it.
//!
//! One line, or none: a tree crew has never worked in says nothing, because
//! an opening line that reads "nothing yet" on every first run is noise.
use std::path::Path;

use super::session::Session;

/// Chars of the remembered request quoted back.
const ASKED_CAP: usize = 90;

/// The line, or `None` when there is nothing worth saying.
pub(crate) fn line(session: &Session, dir: &Path) -> Option<String> {
    let recall = super::recall::lock(&session.recall);
    let last = recall.latest("");
    let check = recall.latest("check: ");
    drop(recall);
    let mut parts: Vec<String> = Vec::new();
    if let Some((text, when)) = last {
        parts.push(format!(
            "last time ({}): {}",
            super::recall::ago_now(when),
            super::thread::clip_chars(asked(&text), ASKED_CAP)
        ));
    }
    if let Some((text, _)) = check {
        parts.push(match answered(&text).starts_with("passed") {
            true => "the check was passing".to_string(),
            false => "the check was FAILING".to_string(),
        });
    }
    match dirty(dir) {
        0 => {}
        1 => parts.push("1 file uncommitted".into()),
        n => parts.push(format!("{n} files uncommitted")),
    }
    (!parts.is_empty()).then(|| parts.join(" \u{b7} "))
}

/// The request half of a stored turn (`you asked: …`).
fn asked(text: &str) -> &str {
    text.lines()
        .next()
        .map(|l| l.strip_prefix("you asked: ").unwrap_or(l))
        .unwrap_or(text)
        .trim()
}

/// The answer half (`crew answered: …`).
fn answered(text: &str) -> &str {
    text.lines()
        .nth(1)
        .map(|l| l.strip_prefix("crew answered: ").unwrap_or(l))
        .unwrap_or("")
        .trim()
}

/// Paths that differ from HEAD, or zero when this is not a git tree — the
/// same silence every other git reader here keeps outside a repository.
fn dirty(dir: &Path) -> usize {
    let Ok(head) = super::changed::head_tree(dir) else {
        return 0;
    };
    super::changed::since(dir, &head).map_or(0, |c| c.len())
}

#[cfg(test)]
#[path = "opening_tests.rs"]
mod tests;
