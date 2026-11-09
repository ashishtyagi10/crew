//! When a turn has stopped making progress, from what its rounds came back
//! with — the second moment the advisor is for, after a failed check.
//!
//! WHY: a turn that re-reads the files it has, or sends an edit that already
//! failed, gets nothing new from the round and spends one of its budget on
//! it; nothing outside the agent ever said so. The evidence is already in the
//! harness: a call answered by [`Seen`] with a pointer to its first result
//! (or the first failure of the same edit), or a call whose key ran before
//! this turn and came back with the very same text — the same error twice
//! included. A round all of whose calls are like that told the agent
//! nothing. One such round is enough to ask: the model's own repeat is the
//! signal, so no count of rounds is tuned here.
//!
//! The checklist (`sys:todo`) is neither progress nor a repeat: ticking a
//! step off beside a re-read does not make the re-read new.

use std::collections::HashMap;

use crate::tools::seen::{key, Seen};
use crate::tools::{todo, ToolCall};

/// What a turn's calls came back with, by call, and whether the stall now
/// going on has been asked about.
#[derive(Default)]
pub(super) struct Progress {
    results: HashMap<String, String>,
    asked: bool,
}

impl Progress {
    /// Note one round, and whether it is a stall not yet asked about: a round
    /// that learned something lets the next stall be asked about again.
    pub(super) fn round(&mut self, calls: &[ToolCall], ran: &[(bool, String)]) -> bool {
        let mut counted = 0;
        let mut stale = true;
        for (call, (_, out)) in calls.iter().zip(ran) {
            if todo::is_todo(call) {
                continue;
            }
            counted += 1;
            let k = key(call);
            let same = Seen::answered_before(out) || self.results.get(&k) == Some(out);
            stale &= same;
            if !Seen::answered_before(out) {
                self.results.insert(k, out.clone());
            }
        }
        let stalled = counted > 0 && stale;
        if !stalled {
            self.asked = false;
            return false;
        }
        !std::mem::replace(&mut self.asked, true)
    }
}

#[cfg(test)]
#[path = "stall_tests.rs"]
mod tests;
