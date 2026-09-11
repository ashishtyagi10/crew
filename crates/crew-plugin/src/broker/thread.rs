//! The pane's short-term memory: the last few turns of THIS session — what
//! was asked, what crew answered — kept in the broker's `Session`.
//!
//! WHY: every plain message used to start from nothing — a fresh graph, a
//! fresh relay thread — so "now do the same for the tests" or "shorter"
//! reached the model with no idea what "the same" or "it" was. The session
//! log on disk is the durable record and `/resume` folds it in ONCE, on
//! request; this is the always-on thread a follow-up builds on. Bounded
//! twice — [`TURNS_MAX`] turns and [`BYTES_MAX`] bytes, oldest dropped —
//! because it rides in front of every task and a runaway answer must never
//! crowd the request out; the rendered block ([`Thread::context`]) is
//! budgeted again at [`CONTEXT_CAP`], newest turns served first, so the
//! freshest context is the last clipped. `/stop` and a restart clear it.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

/// Turns kept, newest last. Six is a conversation, not a transcript.
pub(crate) const TURNS_MAX: usize = 6;
/// Bytes kept across all turns — a few long answers, not a session.
pub(crate) const BYTES_MAX: usize = 6 * 1024;
/// Chars of one answer kept at record time, so ONE long answer cannot be
/// the whole thread; the marker says what was cut.
pub(crate) const ANSWER_CAP: usize = 3 * 1024;
/// Chars the rendered block may take in front of a task (mirrors
/// `memory::MEM_CAP`, the other standing block a task carries).
pub(crate) const CONTEXT_CAP: usize = 2048;
/// Chars of the last request the router is shown (`World::recent`).
pub(crate) const RECENT_CAP: usize = 160;
const HEAD: &str = "Earlier in this conversation:";

/// The slot the session owns, shared with its worker-thread snapshots.
pub(crate) type SharedThread = Arc<Mutex<Thread>>;

/// One exchange: the user's raw message and the answer the pane showed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Turn {
    pub asked: String,
    pub answered: String,
    /// Unix-epoch milliseconds when the answer landed.
    pub when_ms: u64,
}

/// The recent turns, oldest first.
#[derive(Debug, Default)]
pub(crate) struct Thread {
    turns: VecDeque<Turn>,
}

impl Thread {
    /// Keep one finished turn; an empty side is not a turn. Evicts the
    /// oldest past [`TURNS_MAX`] or [`BYTES_MAX`] (the newest always stays).
    pub(crate) fn record(&mut self, asked: &str, answered: &str) {
        let (asked, answered) = (asked.trim(), answered.trim());
        if asked.is_empty() || answered.is_empty() {
            return;
        }
        self.turns.push_back(Turn {
            asked: asked.to_owned(),
            answered: clip_chars(answered, ANSWER_CAP),
            when_ms: now_ms(),
        });
        while self.turns.len() > TURNS_MAX || (self.bytes() > BYTES_MAX && self.turns.len() > 1) {
            self.turns.pop_front();
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.turns.len()
    }

    #[cfg(test)]
    pub(crate) fn turns(&self) -> impl Iterator<Item = &Turn> {
        self.turns.iter()
    }

    pub(crate) fn clear(&mut self) {
        self.turns.clear();
    }

    fn bytes(&self) -> usize {
        self.turns
            .iter()
            .map(|t| t.asked.len() + t.answered.len())
            .sum()
    }

    /// The last request, flattened to one line for the router's world.
    pub(crate) fn recent(&self) -> Option<String> {
        self.turns
            .back()
            .map(|t| super::route::clip(&t.asked, RECENT_CAP))
    }

    /// The block a task carries, or `None` when there is nothing to carry
    /// (the task then passes through byte-identical). Newest first, each turn
    /// takes an even share of what is left — or, when that share is too small
    /// to render, all of it, so the freshest turn wins a tight budget; a turn
    /// that then cannot fit is counted in one `omitted` line, never silent.
    pub(crate) fn context(&self, budget: usize) -> Option<String> {
        if self.turns.is_empty() {
            return None;
        }
        let mut blocks: Vec<String> = Vec::new();
        let mut omitted = 0usize;
        let mut left = budget.saturating_sub(HEAD.len() + 32); // 32: the omitted line
        let n = self.turns.len();
        for (i, t) in self.turns.iter().rev().enumerate() {
            let share = left / (n - i);
            match render(t, share).or_else(|| render(t, left)) {
                Some(b) => {
                    left = left.saturating_sub(b.chars().count() + 2);
                    blocks.push(b);
                }
                None => omitted += 1,
            }
        }
        blocks.reverse();
        let mut out = String::from(HEAD);
        if omitted > 0 {
            out.push_str(&format!("\n\u{2026} [{omitted} earlier turn(s) omitted]"));
        }
        for b in blocks {
            out.push_str("\n\n");
            out.push_str(&b);
        }
        Some(out)
    }
}

/// One turn under `share` chars: the request keeps up to half, the answer the
/// rest (two clip markers' worth held back); too small for the labels, nothing.
fn render(t: &Turn, share: usize) -> Option<String> {
    const LABELS: usize = "you asked: \ncrew answered: ".len();
    let body = share.checked_sub(LABELS + 64).filter(|b| *b >= 48)?;
    let ask_max = t.asked.chars().count().min(body / 2);
    let asked = clip_chars(&t.asked, ask_max);
    let answered = clip_chars(&t.answered, body.saturating_sub(asked.chars().count()));
    Some(format!("you asked: {asked}\ncrew answered: {answered}"))
}

/// The head of `s`, at most `max` CHARS (never split bytes), marked when cut.
pub(crate) fn clip_chars(s: &str, max: usize) -> String {
    let total = s.chars().count();
    if total <= max {
        return s.to_owned();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}\u{2026} [clipped {} chars]", total - max)
}

fn now_ms() -> u64 {
    let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    since.map(|d| d.as_millis() as u64).unwrap_or_default()
}

/// The shared slot, poison-tolerant.
pub(crate) fn lock(thread: &SharedThread) -> MutexGuard<'_, Thread> {
    thread.lock().unwrap_or_else(|e| e.into_inner())
}

/// `task` behind the thread's block (under [`CONTEXT_CAP`]); byte-identical when empty.
pub(crate) fn with_context(thread: &SharedThread, task: &str) -> String {
    match lock(thread).context(CONTEXT_CAP) {
        Some(block) => format!("{block}\n\nNow:\n{task}"),
        None => task.to_owned(),
    }
}

/// The arms' one recording call: `None` (a failed or cancelled turn) records nothing.
pub(crate) fn record(thread: &SharedThread, asked: &str, answered: Option<String>) {
    if let Some(a) = answered {
        lock(thread).record(asked, &a);
    }
}

/// A fan's replies as one answer: none → nothing to record, one → its text,
/// several → each under its agent's name.
pub(crate) fn combined(replies: Vec<(String, String)>) -> Option<String> {
    match replies.as_slice() {
        [] => None,
        [(_, one)] => Some(one.clone()),
        many => Some(
            many.iter()
                .map(|(name, text)| format!("{name}: {text}"))
                .collect::<Vec<_>>()
                .join("\n\n"),
        ),
    }
}

/// `/doctor`'s line: the mark and the detail.
pub(crate) fn doctor_line(turns: usize) -> (char, String) {
    let mark = if turns > 0 { '\u{2713}' } else { '\u{2013}' };
    let s = if turns == 1 { "" } else { "s" };
    (mark, format!("{turns} turn{s} remembered"))
}

#[cfg(test)]
#[path = "thread_tests.rs"]
mod tests;
