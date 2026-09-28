//! The reads a turn has already made, so the same one is not paid for twice.
//!
//! Measured live: a routed reply spent its turn reading one file over and
//! over, and specialists re-ran the same grep. Nothing noticed. Every repeat
//! ran, its whole result (up to 6,000 chars) went into the next prompt again,
//! and a round of the budget was gone. A read made twice with nothing written
//! in between cannot return anything new, and the loops keep the first result
//! in front of the model, so the repeat is answered with a pointer to it
//! instead. The native path resends every turn whole, so its pointer is always
//! true; the relay and the text path shorten all but their last two results
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
    pub fn ran(&mut self, tools: &dyn Tools, call: &ToolCall, round: u32, ok: bool) {
        if !tools.repeatable(&call.server, &call.tool) {
            self.rounds.clear();
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
