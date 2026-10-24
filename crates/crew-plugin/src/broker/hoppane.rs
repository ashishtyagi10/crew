//! What a hop's turn shows and keeps while it runs: the harness's events
//! into the hop's stream ([`Pane`]), and what the turn ran up ([`Tally`]).
use std::sync::{Arc, Mutex, MutexGuard};

use crew_hive::harness::Events;
use crew_hive::tools::todowire;
use crew_hive::HiveEvent;

use crate::broker::adapter::{HopStream, Usage};
use crate::broker::hop::RunStats;

/// Chars of a result the tool line keeps for its click-open view.
const RESULT_KEEP: usize = 2_000;

/// What one turn ran up, kept apart while it runs and added to the thread's
/// stats when it ends.
#[derive(Default)]
pub(super) struct Tally {
    /// What the turn's model calls and its advisor cost.
    pub(super) stats: RunStats,
    /// The latest call's usage, an earlier fallback kept (`Usage::latest`).
    pub(super) usage: Usage,
    /// Tool calls made, for the note under a turn stopped or failed after one.
    pub(super) calls: u32,
    /// Reply chars streamed over every call of the turn: the hop's running
    /// token estimate, which only grows, so the tick gate never swallows a
    /// short follow-up after a long first reply.
    pub(super) chars: u64,
    /// A checklist write between its call and its result: it goes out as the
    /// plan once accepted, and not at all when refused.
    pub(super) todo: Option<HiveEvent>,
    /// A CLI agent's own words for why its last call failed, for the error
    /// card as its dial would have shown them.
    pub(super) said: Option<String>,
    /// What the latest reply that asked for tools wrote above its calls: an
    /// answer a failure or a stop cut short keeps it, rather than ending on
    /// a note alone.
    pub(super) wrote: String,
}

impl Tally {
    /// One call's reply landed: its usage, and what it wrote with any calls.
    pub(super) fn landed(&mut self, sent: usize, c: &crew_hive::Completion, used: Usage) {
        self.stats.add((sent + c.text.len()) / 4, &used);
        self.usage.latest(used);
        match crew_hive::tools::split_tool_calls(&c.text) {
            Some((above, _)) => self.wrote = above,
            None if !c.calls.is_empty() => self.wrote = c.text.clone(),
            None => {}
        }
    }
}

pub(super) type Shared = Arc<Mutex<Tally>>;

pub(super) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The harness's events, into the hop's stream: reply text and reasoning as
/// they arrive, each of crew's tool calls and its result on the tool lane the
/// pane draws Claude Code's on, so it cannot tell the two apart, and a
/// checklist write as the plan, never as a tool line (`hoptool`). The answer,
/// the failure and the bills are not forwarded: the hop's reply, its error
/// hop and the model's usage say those.
pub(super) struct Pane {
    pub(super) stream: HopStream,
    pub(super) tally: Shared,
}

impl Events for Pane {
    fn publish(&self, ev: HiveEvent) {
        match ev {
            HiveEvent::OutputDelta { text, .. } => {
                (self.stream.on_text)(&text);
                let chars = {
                    let mut t = lock(&self.tally);
                    t.chars += text.chars().count() as u64;
                    t.chars
                };
                (self.stream.on_tokens)(chars / 4);
            }
            HiveEvent::ThoughtDelta { text, .. } if !text.is_empty() => {
                (self.stream.on_thought)(&text);
            }
            HiveEvent::ToolCall { ref label, .. } if todowire::is_list(label) => {
                let mut t = lock(&self.tally);
                t.calls += 1;
                t.todo = Some(ev);
            }
            // A refused write changed nothing: the old list stands.
            HiveEvent::ToolResult { ref label, ok, .. } if todowire::is_list(label) => {
                let held = lock(&self.tally).todo.take();
                if let (true, Some(call)) = (ok, held) {
                    (self.stream.on_tool)(call);
                }
            }
            HiveEvent::ToolCall { .. } => {
                lock(&self.tally).calls += 1;
                (self.stream.on_tool)(ev);
            }
            HiveEvent::ToolResult {
                agent,
                label,
                ok,
                text,
                ms,
            } => {
                lock(&self.tally).stats.approx_tokens += text.len() / 4;
                (self.stream.on_tool)(HiveEvent::ToolResult {
                    agent,
                    label,
                    ok,
                    text: kept(&text),
                    ms,
                });
            }
            _ => {}
        }
    }
}

/// The head of a result the tool line keeps, marked when there was more.
fn kept(text: &str) -> String {
    let mut kept: String = text.trim_end().chars().take(RESULT_KEEP).collect();
    if text.trim_end().chars().count() > RESULT_KEEP {
        kept.push_str("\n\u{2026}");
    }
    kept
}

#[cfg(test)]
#[path = "hoppane_tests.rs"]
mod tests;
