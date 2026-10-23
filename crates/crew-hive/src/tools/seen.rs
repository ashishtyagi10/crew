//! The reads a turn has already made, so the same one is not paid for twice.
//!
//! Measured live: a routed reply spent its turn reading one file over and
//! over, and specialists re-ran the same grep. Nothing noticed. Every repeat
//! ran, its whole result (up to 6,000 chars) went into the next prompt again,
//! and a round of the budget was gone. A read made twice with nothing written
//! in between cannot return anything new, and the loops keep the first result
//! in front of the model, so the repeat is answered with a pointer to it
//! instead. The native path resends every turn whole, so its pointer is true
//! until a request too long for the model's context has its older results
//! cut, when it starts a new `Seen` (`apiagent::overflow`); the relay and the
//! text path shorten all but their last two results
//! (`exchanges`), and point only while the result is still shown whole
//! (`Exchanges::repeat`).
//!
//! One type, used by all three loops, so the rule (what counts as the same
//! call, and what makes an earlier one stale) exists once.

use std::collections::HashMap;

use serde_json::Value;

use super::{ToolCall, Tools};

/// Reads that succeeded since the last write, by [`key`], with where each was
/// last made. One per turn (relay) or per task (swarm worker): a new turn may
/// follow an edit made anywhere, so nothing carries over. Clone, so a batch of
/// reads started together can be checked against each other before any ends.
#[derive(Debug, Default, Clone)]
pub struct Seen {
    rounds: HashMap<String, u32>,
    /// Edits that failed, by [`key`], with the round and the reason
    /// ([`Seen::missed`]); forgotten when anything is written.
    misses: HashMap<String, (u32, String)>,
}

impl Seen {
    /// The round `call` was last made in, when it is a repeat that need not
    /// run: the same read, with nothing written since.
    pub fn check(&self, call: &ToolCall) -> Option<u32> {
        self.rounds.get(&key(call)).copied()
    }

    /// Note a call that RAN in `round` (counted from 1): the model round on
    /// the native path, the exchange on the text loops, where a round may
    /// hold several (`Exchanges::next_entry`).
    ///
    /// The LATEST round it ran in is the one kept. A read runs a second time
    /// only when its first result has been shortened out of the prompt
    /// (`Exchanges::repeat`), and the new round is where it is whole.
    ///
    /// Only a read the surface vouches for is remembered
    /// ([`Tools::repeatable`]). Anything else, a write, a shell command, an MCP
    /// tool nobody classified, may have changed what every earlier read saw,
    /// so it forgets them all, whether or not it succeeded: a command that
    /// failed halfway may still have written. A failed read is not
    /// remembered, because the file may appear or the server come back, and
    /// trying again after a failure is something a model is right to do.
    ///
    /// The checklist (`sys:todo`) is neither: a second write of it is a new
    /// list, so it is never pointed at, and it touches nothing a read saw, so
    /// an agent that ticks a step off after every read keeps its repeats
    /// answered.
    pub fn ran(&mut self, tools: &dyn Tools, call: &ToolCall, round: u32, ok: bool) {
        if super::todo::is_todo(call) {
            return;
        }
        if !tools.repeatable(&call.server, &call.tool) {
            self.rounds.clear();
            // A failed edit wrote nothing; anything else may have.
            if ok || !all_or_nothing(call) {
                self.misses.clear();
            }
        } else if ok {
            self.rounds.insert(key(call), round);
        }
    }

    /// What the model is given in place of a repeat's result. One line, and
    /// not an error: the call did not fail, its answer is already there.
    pub fn pointer(round: u32) -> String {
        format!(
            "(same call as round {round}, nothing has been written since \u{2014} \
             its result is above)"
        )
    }
}

impl Seen {
    /// The round an edit failed in, and why, when `call` is that same edit
    /// sent again with nothing written since. It can only fail the same way:
    /// an edit is all or nothing, so a miss changed no file. 24 of 28 edits
    /// the 2026-10-22/23 probes sent twice repeated one that had just failed,
    /// 23 of them in the same turn, each a round spent on a known answer.
    pub fn missed(&self, call: &ToolCall) -> Option<(u32, &str)> {
        let (round, why) = self.misses.get(&key(call))?;
        Some((*round, why.as_str()))
    }

    /// Note that `call` failed in `round` with `why` (after [`Seen::ran`]).
    pub fn miss(&mut self, call: &ToolCall, round: u32, why: &str) {
        if all_or_nothing(call) {
            self.misses.insert(key(call), (round, why.to_string()));
        }
    }

    /// What the model is given in place of a failed edit sent again: the
    /// first failure's reason, and the file's rows it showed (`169│ …`),
    /// which are the answer to the repeat as much as to the first.
    pub fn again(round: u32, why: &str) -> String {
        let mut why = why.trim_start_matches("ERROR: ").lines();
        let head: String = why.next().unwrap_or_default().chars().take(240).collect();
        let rows: String = why.take(8).map(|r| format!("\n{r}")).collect();
        format!(
            "ERROR: the same edit failed in round {round}, and nothing has been written since, \
             so it was not sent again: {head} \u{2014} change `old` to what the file holds{rows}"
        )
    }
}

/// A call that writes nothing when it fails: `sys:edit` changes every place
/// it was asked to or none (`sysedit`).
fn all_or_nothing(call: &ToolCall) -> bool {
    call.server == "sys" && call.tool == "edit"
}

/// `server:tool` plus the arguments as JSON with its keys sorted, so a call
/// written `{"path": "a.rs"}` and one written `{"path":"a.rs"}` are the same
/// call. Models vary spacing and key order between rounds for no reason;
/// comparing the raw text would let every one of those through. Arguments that
/// are not JSON are compared as written, trimmed.
fn key(call: &ToolCall) -> String {
    let args = match serde_json::from_str::<Value>(&call.args) {
        Ok(v) => sorted(v).to_string(),
        Err(_) => call.args.trim().to_string(),
    };
    format!("{}:{} {args}", call.server, call.tool)
}

/// `v` with every object's keys in order, at every depth. Done by hand rather
/// than left to `serde_json`'s default map: a dependency that switches on its
/// `preserve_order` feature would quietly make key order count again.
fn sorted(v: Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut pairs: Vec<(String, Value)> =
                m.into_iter().map(|(k, v)| (k, sorted(v))).collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(pairs.into_iter().collect())
        }
        Value::Array(a) => Value::Array(a.into_iter().map(sorted).collect()),
        other => other,
    }
}

#[cfg(test)]
#[path = "seen_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "seenmiss_tests.rs"]
mod miss_tests;
