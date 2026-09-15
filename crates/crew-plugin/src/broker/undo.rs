//! "undo that" — putting the last task's file changes back, in words.
//!
//! Crew has taken a checkpoint before every file-touching task since v0.6.44
//! (`checkpoint`), and getting one back meant knowing a command, asking it to
//! list, reading ordinals and typing `/restore 2`. Four steps, all of them
//! about crew's bookkeeping rather than about your files, at the one moment
//! you are least inclined to read a list: right after an agent did something
//! you did not want.
//!
//! So it is a sentence now. "undo that", "revert the last task", "put it
//! back" — matched DETERMINISTICALLY (no model call, so it works keyless and
//! cannot be reached by a misclassification), answered with what would come
//! back, and applied only on your own confirm word. The same shape the commit
//! draft has: propose, show, wait. Nothing here writes to your tree until you
//! say one more word.
use std::sync::{Arc, Mutex};

use crate::PluginEvent;

use super::relay::msg;
use super::session::Session;
use super::undoask::asks_undo;

/// The snapshot an offer is holding, until it is taken or dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pending {
    pub sha: String,
    pub label: String,
    /// Paths that would change if it were put back.
    pub files: Vec<String>,
}

/// The slot the session owns; shared, since the offer is made on a worker.
pub(crate) type SharedUndo = Arc<Mutex<Option<Pending>>>;

/// Files listed in the preview before the rest are summarised.
const SHOWN: usize = 8;

/// Whether the session is holding an offer.
pub(crate) fn pending(session: &Session) -> bool {
    lock(&session.undo).is_some()
}

pub(crate) fn lock(u: &SharedUndo) -> std::sync::MutexGuard<'_, Option<Pending>> {
    u.lock().unwrap_or_else(|e| e.into_inner())
}

/// The router's one call: everything about undo, decided here.
///
/// Checked before every other gate and before any model call, because this
/// is the only path that puts files back — a "yes" held over an undo offer
/// must never be able to reach a commit draft or a plan instead.
pub(crate) fn gate(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> Option<anyhow::Result<()>> {
    if pending(session) {
        if super::intent::gate::confirm_word(task) {
            return Some(apply(session, emit));
        }
        if super::intent::gate::reject_word(task) {
            return Some(drop_offer(session, emit));
        }
    }
    asks_undo(task).map(|nth| offer(session, nth, emit))
}

/// Answer an undo request: say what would come back, and hold the offer.
pub(crate) fn offer(
    session: &Session,
    nth: usize,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let Ok(dir) = std::env::current_dir() else {
        return say(emit, "undo needs a working directory");
    };
    let items = match super::checkpoint::list(&dir) {
        Ok(items) => items,
        Err(e) => return say(emit, &format!("undo failed: {e}")),
    };
    let Some((sha, label)) = items.get(nth).cloned() else {
        return say(
            emit,
            "nothing to undo \u{2014} a checkpoint is taken before every task \
             that changes files, in a git repository",
        );
    };
    let files: Vec<String> = super::changed::since(&dir, &sha)
        .unwrap_or_default()
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    if files.is_empty() {
        *lock(&session.undo) = None;
        return say(
            emit,
            &format!("nothing to undo \u{2014} your files already match {label}"),
        );
    }
    let preview = preview(&files);
    *lock(&session.undo) = Some(Pending {
        sha,
        label: label.clone(),
        files,
    });
    say(
        emit,
        &format!("undo would put back {label}:\n{preview}\nsay \u{201c}yes\u{201d} to do it"),
    )
}

/// The held offer, applied.
pub(crate) fn apply(
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let Some(p) = lock(&session.undo).take() else {
        return say(emit, "nothing was offered to undo");
    };
    let Ok(dir) = std::env::current_dir() else {
        return say(emit, "undo needs a working directory");
    };
    // `restore` answers with the files it DELETED (created since the
    // snapshot), not the ones it put back — so the count comes from the
    // offer, and the deletions get named the way `/restore` named them.
    match super::checkpoint::restore(&dir, &p.sha) {
        Ok(removed) => say(
            emit,
            &format!(
                "undone \u{2014} {} file(s) back to {}{}",
                p.files.len(),
                p.label,
                super::checkpoint::removed_note(&removed)
            ),
        ),
        Err(e) => say(emit, &format!("undo failed: {e}")),
    }
}

/// The held offer, dropped.
pub(crate) fn drop_offer(
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    *lock(&session.undo) = None;
    say(emit, "left as it is")
}

/// The files, a few named and the rest counted.
fn preview(files: &[String]) -> String {
    let mut lines: Vec<String> = files.iter().take(SHOWN).map(|f| format!("  {f}")).collect();
    if files.len() > SHOWN {
        lines.push(format!("  \u{2026} and {} more", files.len() - SHOWN));
    }
    lines.join("\n")
}

fn say(emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>, text: &str) -> anyhow::Result<()> {
    emit(msg("agent smith", text))
}

#[cfg(test)]
#[path = "undo_tests.rs"]
mod tests;
