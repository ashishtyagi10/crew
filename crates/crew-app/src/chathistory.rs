//! Composer prompt history — Up/Down recall what this pane already sent.
//!
//! Every shell and every agent CLI worth using (codex, claude, opencode) does
//! this, and in the agent pane the arrows were classified for the popups and
//! then dropped on the floor when no popup was open. Retyping a long prompt to
//! change one word was the most manual thing left on the daily path.
//!
//! The MATCHING RULE is not reinvented here: the docked input bar has done
//! zsh-style prefix search since long before this, and two different meanings
//! for Up in one app is exactly the kind of seam this codebase keeps deleting.
//! `histsearch` owns the rule; this owns the per-pane state.
//!
//! Session-scoped on purpose. A pane's history is the conversation it is in,
//! and writing every prompt to disk is a different feature — with a different
//! privacy question — than remembering the last thing you typed.

use crate::histsearch::{next_match, prev_match};

/// How many prompts a pane remembers. Deep enough to reach back through a
/// working session, shallow enough that it is never worth pruning.
const CAP: usize = 100;

/// Submitted prompts, oldest first, plus where the arrows currently are.
#[derive(Default)]
pub(crate) struct History {
    lines: Vec<String>,
    /// Index into `lines` being shown, or `None` when the composer holds the
    /// user's own live text.
    at: Option<usize>,
    /// What was typed when navigation began: both the filter Up/Down match
    /// against and the text restored on the way back down past the newest
    /// entry. Empty matches everything, i.e. plain most-recent-first recall.
    prefix: String,
}

impl History {
    /// The prompts themselves, newest last — the ghost suggestion matches
    /// against these (see `ChatPane::ghost`).
    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Remember a submitted prompt. Blank lines and an immediate repeat are
    /// dropped — Up should reach the last DIFFERENT thing you said, not walk
    /// through five copies of a retried command.
    pub(crate) fn record(&mut self, text: &str) {
        self.edited();
        if text.trim().is_empty() || self.lines.last().is_some_and(|l| l == text) {
            return;
        }
        self.lines.push(text.to_string());
        if self.lines.len() > CAP {
            self.lines.remove(0);
        }
    }

    /// The composer's text is the user's own again — either they typed over a
    /// recalled line or they sent one. Either way the next Up starts a fresh
    /// search from the newest entry, against whatever is there now.
    pub(crate) fn edited(&mut self) {
        self.at = None;
        self.prefix.clear();
    }

    /// Up: the newest older entry starting with what was typed. Returns
    /// whether `input` changed, so a caller can tell "recalled" from "nothing
    /// matched" without comparing strings.
    pub(crate) fn prev(&mut self, input: &mut String) -> bool {
        if self.at.is_none() {
            self.prefix = input.clone();
        }
        let before = self.at.unwrap_or(self.lines.len());
        // No match holds still rather than wrapping: wrapping means one
        // keystroke too many silently hands you the newest prompt.
        let Some(i) = prev_match(&self.lines, &self.prefix, before) else {
            return false;
        };
        self.at = Some(i);
        *input = self.lines[i].clone();
        true
    }

    /// Down: the next newer match, and past the newest, back to what was typed
    /// before the walk began.
    pub(crate) fn next(&mut self, input: &mut String) -> bool {
        let Some(cur) = self.at else { return false };
        match next_match(&self.lines, &self.prefix, cur) {
            Some(i) => {
                self.at = Some(i);
                *input = self.lines[i].clone();
            }
            None => {
                self.at = None;
                *input = std::mem::take(&mut self.prefix);
            }
        }
        true
    }
}

#[cfg(test)]
#[path = "chathistory_tests.rs"]
mod tests;
