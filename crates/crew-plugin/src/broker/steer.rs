//! Steer, don't queue: a message typed while a task runs joins that task's
//! next tool round instead of waiting for the whole turn to end.
//!
//! WHY: the pane held everything typed mid-run until the turn settled, so
//! "also check the tests" or "no, the other file" arrived after the agent had
//! already answered the wrong thing. Amp ("Steer, Don't Queue"), Cursor (a
//! follow-up waits for the next tool call, not the end of the turn) and Codex
//! (`instant_interrupt`) all moved the same way.
//!
//! How: the stdin loop drops a [`crate::PluginCommand::Steer`] here while a
//! task runs; the relay's tool loop (`toolcall::run_tools`) [`take`]s what is
//! waiting between rounds — announcing each as a
//! [`crate::PluginEvent::Steered`] BEFORE it dials, so the host drops its
//! queued copy ahead of the answer — and every follow-up of the turn from
//! then on carries it ([`section`]).
//!
//! Only a thread holding a [`Taking`] takes: the worker the stdin loop spawns
//! for a task. A swarm runs on that thread's runtime, so its workers take
//! through a run-wide log (`crew_hive::steers`, fed by [`take`]) that every
//! worker reads, and the lead's closing answer and the judge read what was
//! taken ([`told`]). A fan's threads and CLI agents (claude/opencode relays),
//! which run their own loops out of reach of this one, never do — their
//! steers go untaken, the last task out empties the inbox, and the host's
//! queued copy is sent when the turn settles, as it always was.
//!
//! One inbox per process, like `approval`'s mailbox: a broker is one session
//! with one stdin. With several tasks running, the first to reach a round
//! takes what is waiting.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::PluginEvent;

/// How a taking thread tells the host a steer joined.
pub(crate) type Emit = Arc<dyn Fn(PluginEvent) + Send + Sync>;

/// The follow-up section's heading, the swarm's too: `crew_hive::steers`
/// formats both ([`section`]). Named here for the tests beside this file.
#[cfg(test)]
use crew_hive::steers::HEAD;

/// The line that closes it: the later word is the one that counts.
pub(crate) const TAIL: &str = "Take this into account from here on \u{2014} where it \
                               differs from the request above, it replaces it.";

/// What is waiting, and how many tasks could take it. One lock over both, so
/// a steer can never be accepted in the instant the last task empties the
/// inbox and then sit there for the next, unrelated task to find.
struct Inbox {
    running: usize,
    offers: VecDeque<(String, String)>,
}

static INBOX: Mutex<Inbox> = Mutex::new(Inbox {
    running: 0,
    offers: VecDeque::new(),
});

thread_local! {
    /// This thread's way to the host while it holds a [`Taking`]; `None` on
    /// every other thread, which is what keeps them from taking.
    static TAKER: RefCell<Option<Emit>> = const { RefCell::new(None) };
    /// Everything this thread's task has taken so far ([`told`]).
    static TAKEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn inbox() -> MutexGuard<'static, Inbox> {
    INBOX.lock().unwrap_or_else(|e| e.into_inner())
}

/// A task in flight on this thread, for as long as it lives. While any is
/// alive a steer is accepted; when the last one drops, whatever no round took
/// is thrown away — the host still holds its copy and sends it next.
///
/// Not `Send`: it must drop on the thread it began on, the thread whose
/// [`TAKER`] it set.
pub(crate) struct Taking {
    _thread_bound: PhantomData<*const ()>,
}

impl Taking {
    /// Mark this thread as a task's worker, announcing taken steers to `emit`.
    pub(crate) fn begin(emit: Emit) -> Self {
        inbox().running += 1;
        TAKER.with(|t| *t.borrow_mut() = Some(emit));
        TAKEN.with(|t| t.borrow_mut().clear());
        Taking {
            _thread_bound: PhantomData,
        }
    }
}

impl Drop for Taking {
    fn drop(&mut self) {
        TAKER.with(|t| *t.borrow_mut() = None);
        let mut g = inbox();
        g.running = g.running.saturating_sub(1);
        if g.running == 0 {
            g.offers.clear();
        }
    }
}

/// Offer `text` to the running task. Ignored when nothing runs — nothing
/// could take it, and a steer left waiting would slip into the NEXT task,
/// which the host is about to start from its own queued copy anyway.
pub(crate) fn deliver(channel: String, text: String) {
    let mut g = inbox();
    if g.running > 0 && !text.trim().is_empty() {
        g.offers.push_back((channel, text));
    }
}

/// Throw away everything waiting (`/stop`: a cancelled run takes nothing more,
/// and the host drops its queue with it).
pub(crate) fn clear() {
    inbox().offers.clear();
}

/// Everything offered since the last take, each announced to the host first.
/// Empty — and the inbox untouched — on a thread holding no [`Taking`].
pub(crate) fn take() -> Vec<String> {
    let Some(emit) = TAKER.with(|t| t.borrow().clone()) else {
        return Vec::new();
    };
    let taken: Vec<(String, String)> = inbox().offers.drain(..).collect();
    let texts: Vec<String> = taken
        .into_iter()
        .map(|(channel, text)| {
            emit(PluginEvent::Steered {
                channel,
                text: text.clone(),
            });
            text
        })
        .collect();
    TAKEN.with(|t| t.borrow_mut().extend(texts.iter().cloned()));
    texts
}

/// What this thread's task was told mid-run, as a section to follow the
/// request in the swarm lead's closing brief and the judge's: the answer is
/// written, and judged, against the request as the user left it. Empty when
/// nothing was taken — the brief is then byte for byte what it was.
pub(crate) fn told() -> String {
    let taken = TAKEN.with(|t| t.borrow().clone());
    match section(&taken) {
        s if s.is_empty() => s,
        s => format!("\n\n{}", s.trim_end()),
    }
}

/// The follow-up section for every steer taken this turn so far (empty when
/// none): after the tool exchanges, where the newest word belongs, and
/// ahead of the line that says how many calls are left. A message of several
/// lines stays one item, its later lines indented under the dash.
pub(crate) fn section(added: &[String]) -> String {
    crew_hive::steers::section(added, TAIL)
}

/// This thread's task as told `said`, for the briefs' own tests, which run
/// with no stdin loop to take from.
#[cfg(test)]
pub(crate) fn told_in_test(said: &[&str]) {
    TAKEN.with(|t| *t.borrow_mut() = said.iter().map(|s| s.to_string()).collect());
}

#[cfg(test)]
#[path = "steer_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "swarmsteer_tests.rs"]
mod swarm_tests;

#[cfg(test)]
#[path = "steerround_tests.rs"]
mod round_tests;
