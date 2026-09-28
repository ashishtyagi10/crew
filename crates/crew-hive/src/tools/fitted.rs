//! The log cut to fit the model's context, for the rest of a turn.
//!
//! Shortening all but the last two rounds (`exchanges`) keeps a long turn
//! cheap, but not bounded: each older round still carries its message and a
//! 400-char head, and the last two rounds of big reads alone can reach a
//! 32K-token window once the base prompt is in front of them. The provider
//! then refuses the call, and the turn used to end there with everything it
//! had read thrown away. So on that refusal the loop cuts once more and
//! dials again: the last round stays whole, since it is what the model asked
//! for and has not seen, and every older round is its `CALLED` line alone. A
//! line per call says what was done; the note under them says the results
//! can be had again, and asking runs the call again (`Exchanges::repeat`).

use super::Exchanges;

/// What the pane says, once, when a loop cuts its request to fit and tries
/// again: the relay as a broker note, a swarm worker in its live card.
pub const CONTEXT_FULL: &str = "context full \u{2014} older results shortened, retrying";

/// The line under the older rounds' calls.
const LEFT_OUT: &str = "(the results of the calls above were left out to fit the model's \
     context \u{2014} make a call again if you need its result)";

impl Exchanges {
    /// Cut the log to fit for the rest of the turn. `false` when that leaves
    /// the prompt as it was, cut already or one round long, so a second try
    /// would send the same request that was just refused.
    pub fn tighten(&mut self) -> bool {
        let before = self.render();
        self.tight = true;
        self.render() != before
    }

    /// The last round whole, every older call as its `CALLED` line.
    pub(super) fn fitted(&self) -> String {
        let last = self.rounds();
        let (old, new): (Vec<_>, Vec<_>) = self.all.iter().partition(|e| e.round < last);
        let mut shown: Vec<String> = Vec::with_capacity(new.len() + 1);
        if !old.is_empty() {
            let calls: Vec<&str> = old.iter().map(|e| e.called.as_str()).collect();
            shown.push(format!("{}\n{LEFT_OUT}", calls.join("\n")));
        }
        shown.extend(new.iter().map(|e| e.to_string()));
        shown.join("\n\n")
    }
}

#[cfg(test)]
#[path = "fitted_tests.rs"]
mod tests;
