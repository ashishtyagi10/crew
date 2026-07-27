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
mod tests {
    use super::*;

    fn hist(lines: &[&str]) -> History {
        let mut h = History::default();
        for l in lines {
            h.record(l);
        }
        h
    }

    #[test]
    fn up_walks_back_and_stops_at_the_oldest() {
        let mut h = hist(&["one", "two"]);
        let mut s = String::new();
        assert!(h.prev(&mut s));
        assert_eq!(s, "two");
        assert!(h.prev(&mut s));
        assert_eq!(s, "one");
        // Oldest: held, not wrapped round to the newest.
        assert!(!h.prev(&mut s));
        assert_eq!(s, "one");
    }

    #[test]
    fn down_walks_forward_and_restores_what_was_typed() {
        let mut h = hist(&["one", "two"]);
        let mut s = String::new();
        h.prev(&mut s);
        h.prev(&mut s);
        assert_eq!(s, "one");
        assert!(h.next(&mut s));
        assert_eq!(s, "two");
        assert!(h.next(&mut s));
        assert_eq!(s, "", "past the newest is the empty composer again");
        assert!(!h.next(&mut s), "already live");
    }

    /// The input bar's rule, in the composer: typed text filters the walk
    /// instead of being thrown away by it.
    #[test]
    fn typed_text_filters_the_recall_and_comes_back() {
        let mut h = hist(&["/model list", "run the tests", "/model all qwen-max"]);
        let mut s = "/model".to_string();
        assert!(h.prev(&mut s));
        assert_eq!(s, "/model all qwen-max");
        assert!(h.prev(&mut s), "skips the non-matching entry between them");
        assert_eq!(s, "/model list");
        assert!(!h.prev(&mut s), "no older /model");
        h.next(&mut s);
        h.next(&mut s);
        assert_eq!(s, "/model", "the typed prefix is restored, not lost");
    }

    #[test]
    fn a_prefix_matching_nothing_leaves_the_composer_alone() {
        let mut h = hist(&["one", "two"]);
        let mut s = "zzz".to_string();
        assert!(!h.prev(&mut s));
        assert_eq!(s, "zzz");
    }

    #[test]
    fn down_from_live_input_does_nothing() {
        let mut h = hist(&["one"]);
        let mut s = "typing".to_string();
        assert!(!h.next(&mut s));
        assert_eq!(s, "typing");
    }

    #[test]
    fn empty_history_recalls_nothing() {
        let mut h = History::default();
        let mut s = "typing".to_string();
        assert!(!h.prev(&mut s));
        assert_eq!(s, "typing");
    }

    #[test]
    fn editing_a_recalled_line_makes_it_the_draft() {
        let mut h = hist(&["one", "two"]);
        let mut s = String::new();
        h.prev(&mut s);
        s.push('!'); // the caller's edit
        h.edited();
        // Down must not resurrect the old prefix over what was just typed.
        assert!(!h.next(&mut s));
        assert_eq!(s, "two!");
    }

    #[test]
    fn blanks_and_immediate_repeats_are_not_recorded() {
        let mut h = hist(&["one", "one", "  ", "", "two"]);
        let mut s = String::new();
        h.prev(&mut s);
        assert_eq!(s, "two");
        h.prev(&mut s);
        assert_eq!(s, "one");
        assert!(!h.prev(&mut s), "only two entries should exist");
    }

    #[test]
    fn a_repeat_that_is_not_immediate_is_kept() {
        let mut h = hist(&["one", "two", "one"]);
        let mut s = String::new();
        h.prev(&mut s);
        assert_eq!(s, "one");
        h.prev(&mut s);
        assert_eq!(s, "two");
    }

    #[test]
    fn recording_returns_the_arrows_to_the_newest() {
        let mut h = hist(&["one", "two"]);
        let mut s = String::new();
        h.prev(&mut s);
        h.prev(&mut s);
        assert_eq!(s, "one");
        h.record("three"); // sending from mid-history
        let mut s = String::new();
        assert!(h.prev(&mut s));
        assert_eq!(s, "three", "Up starts from the newest again");
    }

    #[test]
    fn the_oldest_entries_fall_off_at_the_cap() {
        let mut h = History::default();
        for i in 0..CAP + 10 {
            h.record(&format!("p{i}"));
        }
        assert_eq!(h.lines.len(), CAP);
        let mut s = String::new();
        h.prev(&mut s);
        assert_eq!(s, format!("p{}", CAP + 9), "newest survives");
        // Walking to the end reaches the oldest entry still held, not p0.
        while h.prev(&mut s) {}
        assert_eq!(s, "p10");
    }
}
