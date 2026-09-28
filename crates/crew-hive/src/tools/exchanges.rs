//! A turn's tool exchanges, and how much of each one the next prompt carries.
//!
//! The relay and the swarm's `@tool` path rebuild the whole follow-up prompt
//! after every round, and every earlier result went into it again, whole. By
//! the eighth round of a solo turn that was eight results of up to 6,000 chars
//! on top of an ~11 K base prompt, ~60 K chars re-sent, billed and waited on
//! each round, while the model works from the last result or two and needs
//! only a reminder of the rest. So the last [`WHOLE`] exchanges keep their
//! results whole, and every older one keeps what the agent wrote and its
//! CALLED line but only the head of its result, with a line saying how long it
//! was and that the call can be made again.
//!
//! One type for both loops, so the rule, and what it means for a repeat that
//! [`Seen`] would answer with a pointer, exist once. The native path is not
//! here: it resends every turn whole, and shortening those is not done yet.

use super::seen::Seen;
use super::ToolCall;

/// Exchanges at the end of the log whose results are always carried whole:
/// the one the model is answering, and the one before, which it is often
/// still comparing against. It is also what keeps a turn of one or two calls
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
    /// The `YOUR MESSAGE:` block, the `CALLED` line and the `RESULT:` heading.
    head: String,
    /// The result as the loop clipped it to its own cap.
    result: String,
}

impl Exchange {
    /// `said` is the reply with its call cut out ([`super::said`]); `result`
    /// arrives already clipped, because the two loops clip differently (the
    /// relay keeps a result's last line, where `sys:read_file` says how to
    /// continue).
    pub fn new(said: &str, label: &str, args: &str, result: String) -> Self {
        Self {
            head: format!("{}CALLED {label} {args}\nRESULT:\n", super::said(said)),
            result,
        }
    }

    /// Whether the result is long enough to be shortened once it is older.
    fn long(&self) -> bool {
        self.result.chars().count() > SHORT
    }

    /// The exchange with its result cut to its head and a note.
    fn shortened(&self) -> String {
        format!(
            "{}{}\n\u{2026} (result shortened \u{2014} {} chars; the call can be made again \
             to see it all)",
            self.head,
            super::said::head_lines(&self.result, HEAD),
            grouped(self.result.chars().count())
        )
    }
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.head, self.result)
    }
}

/// Every exchange of one turn (relay) or one task (swarm worker), in order.
/// Round N is the Nth entry: each round records exactly one.
#[derive(Debug, Default)]
pub struct Exchanges(Vec<Exchange>);

impl Exchanges {
    pub fn push(&mut self, e: Exchange) {
        self.0.push(e);
    }

    /// The log as the next prompt shows it: the last [`WHOLE`] exchanges and
    /// every short one whole, the rest shortened.
    pub fn render(&self) -> String {
        let from = self.0.len().saturating_sub(WHOLE);
        let shown: Vec<String> = self
            .0
            .iter()
            .enumerate()
            .map(|(i, e)| match i < from && e.long() {
                true => e.shortened(),
                false => e.to_string(),
            })
            .collect();
        shown.join("\n\n")
    }

    /// The round `seen` would point `call` at, while that pointer is true.
    ///
    /// The pointer says the result "is above", and in the prompt that will
    /// carry it that is only so while the round is still shown whole: one of
    /// the last [`WHOLE`] once this call's own exchange is added, or a short
    /// one. Past that, the repeat RUNS again, rather than being answered from
    /// a stored copy of the whole result. Keeping one would mean holding a
    /// second copy of every read for the turn for the one case the model asks,
    /// and the shortened line invites exactly that call, so the answer should
    /// be a real one: a read costs a file read, not a model call, and it sees
    /// the file as it is now, which a copy would not if it changed behind
    /// crew's back. [`Seen::ran`] then points later repeats at the new round.
    pub fn repeat(&self, seen: &Seen, call: &ToolCall) -> Option<u32> {
        seen.check(call).filter(|&round| self.whole_next(round))
    }

    /// Whether round `round` (from 1) is still shown whole once one more
    /// exchange is added.
    fn whole_next(&self, round: u32) -> bool {
        let Some(i) = (round as usize).checked_sub(1) else {
            return false;
        };
        i + WHOLE > self.0.len() || self.0.get(i).is_some_and(|e| !e.long())
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
