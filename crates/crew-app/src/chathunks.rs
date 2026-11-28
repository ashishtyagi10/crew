//! Answering a file-edit question hunk by hunk, and wording it in the transcript.
//!
//! The question carries the diff as numbered hunks (the broker's `hunkshow`). Enter allows all and
//! Esc refuses all as for any question; typing `1 3`, `1,3` or `-2` and sending keeps only those.
//! A number that is not a hunk is said so and the question STAYS OPEN: guessing what was meant
//! would allow a change the person did not pick.
use crew_plugin::hunks::{declared, parse_picks, Picks};
use crew_plugin::PluginCommand;

use crate::chat::ChatPane;

/// The transcript line when a question arrives. A diff question is its first line, the rows in a
/// `diff` fence (so `-`/`+` rows wear the transcript's diff colours), and its keys line below.
pub(crate) fn asked(id: &str, question: &str) -> String {
    let rows: Vec<&str> = question.lines().collect();
    match rows.as_slice() {
        // Only the broker's own diff question ends in its keys line; a multi-line command is
        // not a diff and keeps its plain wording.
        [head, diff @ .., keys]
            if !diff.is_empty() && keys.starts_with("Enter ") && !crate::spendask::is_spend(id) =>
        {
            format!(
                "\u{26a0} allow this? {head}\n````diff\n{}\n````\n{keys}",
                diff.join("\n")
            )
        }
        _ => crate::spendask::asked(id, question),
    }
}

/// The transcript line once it is answered: the question's first line, not its whole diff again.
fn answered(id: &str, question: &str, granted: bool, kept: Option<&[u32]>) -> String {
    let head = question.lines().next().unwrap_or_default();
    match kept {
        Some(k) => {
            let k: Vec<String> = k.iter().map(u32::to_string).collect();
            format!("allowed hunks {} only \u{2014} {head}", k.join(", "))
        }
        None => crate::spendask::answered(id, head, granted),
    }
}

impl ChatPane {
    /// The question into the transcript, OPEN: a long note folds to a `… +13` stub, and a diff
    /// folded away is the very thing the person was asked to read.
    pub(crate) fn push_asked(&mut self, id: &str, question: &str) {
        self.push_note(asked(id, question));
        if let Some(m) = self.messages.last_mut() {
            m.expanded = true;
        }
    }

    /// Answer the question [`Self::asking`] names, allowing only `hunks` of a file edit when
    /// given. `false` when none waits.
    pub(crate) fn answer_with(&mut self, granted: bool, hunks: Option<Vec<u32>>) -> bool {
        let timeout = crew_plugin::approval::DEFAULT_TIMEOUT_MS;
        let now = crate::anim::now_ms();
        self.asking
            .retain(|a| now.saturating_sub(a.since) < timeout);
        let Some(a) = self.asking.pop_front() else {
            return false;
        };
        let note = answered(&a.id, &a.question, granted, hunks.as_deref());
        let _ = self.plugin.send(&PluginCommand::Approve {
            id: a.id,
            granted,
            hunks,
        });
        self.push_note(note);
        true
    }

    /// The typed text as hunk picks, when a file edit is waiting and it reads as ones. `true` when
    /// it was consumed: answered, or refused with the reason said and the question kept.
    pub(crate) fn take_picks(&mut self) -> bool {
        let Some(n) = self.asking().map(|a| declared(&a.question)) else {
            return false;
        };
        match parse_picks(self.input.trim(), n) {
            Picks::NotPicks => false,
            Picks::Bad(why) => {
                self.input.clear();
                self.push_note(why);
                true
            }
            Picks::Keep(k) => {
                self.input.clear();
                let all = k.len() == n;
                self.answer_with(true, (!all).then_some(k));
                true
            }
        }
    }
}

#[cfg(test)]
#[path = "chathunks_tests.rs"]
mod tests;
