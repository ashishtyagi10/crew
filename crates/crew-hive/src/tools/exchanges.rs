//! A turn's tool exchanges, and how much of each one the next prompt carries.
//!
//! The relay and the swarm's `@tool` path rebuild the whole follow-up prompt
//! after every round, and every earlier result went into it again, whole. By
//! the eighth round of a solo turn that was eight results of up to 6,000 chars
//! on top of an ~11 K base prompt, ~60 K chars re-sent, billed and waited on
//! each round, while the model works from the last result or two and needs
//! only a reminder of the rest. So the exchanges of the last [`WHOLE`] rounds
//! keep their results whole, and every older one keeps what the agent wrote
//! and its CALLED line but only the head of its result, with a line saying how
//! long it was and that the call can be made again.
//!
//! Rounds, not exchanges: one reply may make several reads (`split_tool_calls`),
//! each its own exchange, and counting those alone would shorten the first of
//! three files in the very prompt that answers it.
//!
//! One type for both loops, so the rule, and what it means for a repeat that
//! [`Seen`] would answer with a pointer, exist once. A turn that outgrows the
//! model's context anyway is cut harder (`fitted`). The native path is not
//! here: it resends its turns as turns and cuts them itself (`apiagent`).

use super::seen::Seen;
use super::ToolCall;

#[path = "fitted.rs"]
mod fitted;
pub use fitted::CONTEXT_FULL;

/// Rounds at the end of the log whose results are always carried whole: the
/// one the model is answering, and the one before, which it is often still
/// comparing against. It is also what keeps a turn of one or two calls
/// byte-identical to what it was before anything was shortened.
const WHOLE: usize = 2;

/// Chars of an older result's head that are kept: enough to say what the call
/// found (the first lines of a file, the first hits of a grep).
const HEAD: usize = 400;

/// Results at most this long are never shortened. The head and the note would
/// save next to nothing, and a short result (an error, "no matches") is often
/// the whole reason for the next call.
const SHORT: usize = 600;

/// One round of the log, kept in its two parts so the result can be shortened
/// when a prompt is built, rather than joined once and carried whole forever.
#[derive(Debug, Clone)]
pub struct Exchange {
    /// The `YOUR MESSAGE:` block, empty when the reply was only the call.
    said: String,
    /// `CALLED <label> <args>`, kept apart because a log cut to fit shows an
    /// older round as this line alone (`fitted`).
    called: String,
    /// The result as the loop clipped it to its own cap.
    result: String,
    /// The round it was made in, from 1, given when it is pushed.
    round: u32,
}

impl Exchange {
    /// `said` is the reply with its call cut out ([`super::said`]); `result`
    /// arrives already clipped, because the two loops clip differently (the
    /// relay keeps a result's last line, where `sys:read_file` says how to
    /// continue).
    pub fn new(said: &str, label: &str, args: &str, result: String) -> Self {
        Self {
            said: super::said(said),
            called: format!("CALLED {label} {args}"),
            result,
            round: 0,
        }
    }

    /// Whether the result is long enough to be shortened once it is older.
    fn long(&self) -> bool {
        self.result.chars().count() > SHORT
    }

    /// The exchange with its result cut to its head and a note.
    fn shortened(&self) -> String {
        format!(
            "{}{}\nRESULT:\n{}\n\u{2026} (result shortened \u{2014} {} chars; the call can \
             be made again to see it all)",
            self.said,
            self.called,
            super::said::head_lines(&self.result, HEAD),
            grouped(self.result.chars().count())
        )
    }
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}\nRESULT:\n{}", self.said, self.called, self.result)
    }
}

/// Every exchange of one turn (relay) or one task (swarm worker), in order:
/// one per call, so a round that made three reads records three.
#[derive(Debug, Default)]
pub struct Exchanges {
    all: Vec<Exchange>,
    /// Cut to fit the model's context: every round but the last shown as its
    /// `CALLED` lines alone, from the moment it was cut to the end of the turn.
    tight: bool,
}

impl Exchanges {
    /// Record a round that made one call.
    pub fn push(&mut self, e: Exchange) {
        self.push_round(vec![e]);
    }

    /// Record a round: every call one reply made, in the order written.
    pub fn push_round(&mut self, calls: Vec<Exchange>) {
        let round = self.rounds() + 1;
        self.all
            .extend(calls.into_iter().map(|e| Exchange { round, ..e }));
    }

    /// The number the next exchange pushed will have, from 1: what a loop
    /// hands [`Seen::ran`], so a pointer finds the one call it stands for
    /// when a round holds several.
    pub fn next_entry(&self) -> u32 {
        self.all.len() as u32 + 1
    }

    /// Rounds recorded so far.
    fn rounds(&self) -> u32 {
        self.all.last().map_or(0, |e| e.round)
    }

    /// The log as the next prompt shows it: the last [`WHOLE`] rounds and
    /// every short exchange whole, the rest shortened; once cut to fit, the
    /// last round whole and the rest as their calls (`fitted`).
    pub fn render(&self) -> String {
        if self.tight {
            return self.fitted();
        }
        let old = self.rounds().saturating_sub(WHOLE as u32);
        let shown: Vec<String> = self
            .all
            .iter()
            .map(|e| match e.round <= old && e.long() {
                true => e.shortened(),
                false => e.to_string(),
            })
            .collect();
        shown.join("\n\n")
    }

    /// The round `seen` would point `call` at, while that pointer is true.
    ///
    /// `seen` holds the entry each read was recorded under ([`next_entry`]).
    /// The pointer says the result "is above", and in the prompt that will
    /// carry it that is only so while the entry is still shown whole: in one
    /// of the last [`WHOLE`] rounds once this call's own round is added, or a
    /// short one; an entry not pushed yet is in the round being built, beside
    /// this call. Past that, the repeat RUNS again, rather than being answered
    /// from a stored copy of the whole result. Keeping one would mean holding
    /// a second copy of every read for the turn for the one case the model
    /// asks, and the shortened line invites exactly that call, so the answer
    /// should be a real one: a read costs a file read, not a model call, and
    /// it sees the file as it is now, which a copy would not if it changed
    /// behind crew's back. [`Seen::ran`] then points later repeats at the new
    /// round.
    ///
    /// Once cut to fit, no older result is shown at all, short or long, so
    /// only a repeat within the round being built is pointed at.
    ///
    /// [`next_entry`]: Exchanges::next_entry
    pub fn repeat(&self, seen: &Seen, call: &ToolCall) -> Option<u32> {
        let i = (seen.check(call)? as usize).checked_sub(1)?;
        let next = self.rounds() + 1;
        let whole = if self.tight { 1 } else { WHOLE as u32 };
        match self.all.get(i) {
            None => Some(next),
            Some(e) => (e.round + whole > next || (!self.tight && !e.long())).then_some(e.round),
        }
    }
}

/// `5812` → `5,812`: the count is read by a model, and by a person looking at
/// a prompt, and both read a grouped number at a glance.
fn grouped(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
#[path = "exchanges_tests.rs"]
mod tests;
