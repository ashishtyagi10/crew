//! The notification system's pure core: a typed event kind, a recorded
//! notification, and a [`Notifier`] that throttles duplicates and formats the
//! one-line message surfaced on the input bar + sidebar LOG. No rendering, no
//! PTY, no clock — `record` takes `now` so it stays deterministic and testable.
//! Detection lives in `poll.rs`; surfacing in `status.rs`/`app.rs`.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// What happened in a pane. Each maps to a distinct config toggle and message.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotifyKind {
    /// A foreground command returned to the shell prompt after `notify_min_secs`.
    AgentDone,
    /// …and the shell reported it exited non-zero (OSC 133 `D`). The same
    /// event as [`Self::AgentDone`] and the same switch — a failure is a
    /// finish — told apart because "it is done" and "it went wrong" are not
    /// the same news, and only one of them is worth getting up for.
    Failed,
    /// A program rang the terminal bell.
    Bell,
    /// A watched substring appeared in the pane's output.
    Pattern,
    /// The pane's process exited.
    Exited,
    /// The program in the pane asked for one itself (OSC 9 / OSC 777).
    Requested,
    /// The pane is blocked on the user — an approval prompt, a y/n question,
    /// or a pending plan (see `blocked.rs`).
    Waiting,
}

/// A recorded notification, kept in a small ring for throttling and `/notify`.
#[derive(Clone, Debug)]
pub struct Notification {
    pub kind: NotifyKind,
    /// Human label of the originating pane (its title).
    pub pane: String,
    /// Event-specific detail: the finished command, the matched pattern, etc.
    pub detail: String,
    pub at: Instant,
}

/// Most notifications kept for throttling + `/notify` listing (oldest dropped).
const CAP: usize = 32;

/// An identical (kind, pane, detail) event within this window is suppressed, so
/// a chatty pattern or a spammy bell can't flood the LOG.
const COOLDOWN: Duration = Duration::from_secs(10);

/// Throttling ring buffer over recent notifications.
#[derive(Default)]
pub struct Notifier {
    recent: VecDeque<Notification>,
}

impl Notifier {
    /// Record an event at `now`. Returns the formatted one-line message to flash
    /// and log, or `None` when throttled (an identical event within `COOLDOWN`).
    pub fn record(
        &mut self,
        kind: NotifyKind,
        pane: String,
        detail: String,
        now: Instant,
    ) -> Option<String> {
        let throttled = self.recent.iter().any(|n| {
            n.kind == kind
                && n.pane == pane
                && n.detail == detail
                && now.saturating_duration_since(n.at) < COOLDOWN
        });
        if throttled {
            return None;
        }
        let msg = format_message(kind, &pane, &detail);
        self.recent.push_back(Notification {
            kind,
            pane,
            detail,
            at: now,
        });
        while self.recent.len() > CAP {
            self.recent.pop_front();
        }
        Some(msg)
    }

    /// Number of notifications currently retained (for `/notify`).
    pub fn len(&self) -> usize {
        self.recent.len()
    }
}

/// Result of evaluating a foreground-command transition: whether a "finished"
/// event should fire (carrying the finished command's name) and the updated
/// `cmd_since` start time to store back on the pane.
#[derive(Debug, PartialEq, Eq)]
pub struct AgentDone {
    /// `Some(command)` when a finished notification should fire.
    pub finished: Option<String>,
    /// The new value for the pane's `cmd_since`.
    pub since: Option<Instant>,
}

/// Decide whether a foreground-command change is a "command finished" event. The
/// foreground command went from `old` to `new`; `since` is when the current
/// command started. A finished event fires only when a command returns to the
/// idle prompt (`Some → None`) after running at least `min`. Pure: `now` is
/// injected so it can be tested without a clock.
pub fn agent_done(
    old: Option<&str>,
    new: Option<&str>,
    since: Option<Instant>,
    min: Duration,
    now: Instant,
) -> AgentDone {
    match (old, new) {
        // A command launched at the idle prompt: start the timer.
        (None, Some(_)) => AgentDone {
            finished: None,
            since: Some(now),
        },
        // Returned to the prompt: fire iff it ran long enough and we saw it start.
        (Some(cmd), None) => {
            let ran = since.map(|s| now.saturating_duration_since(s));
            let long_enough = ran.is_some_and(|d| d >= min);
            AgentDone {
                // How long it took is half the news — a build that finished in
                // six seconds and one that took nine minutes are different
                // events, and the notification said the same thing for both.
                finished: long_enough.then(|| match ran.and_then(crate::runclock::label) {
                    Some(t) => format!("{cmd} ({t})"),
                    None => cmd.to_string(),
                }),
                since: None,
            }
        }
        // One command launched another (or unchanged): not finished; keep the
        // original start so the whole busy burst is timed as one.
        (Some(_), Some(_)) => AgentDone {
            finished: None,
            since,
        },
        // Still idle.
        (None, None) => AgentDone {
            finished: None,
            since: None,
        },
    }
}

/// The one-line message for an event, e.g. `claude finished in crew`. Kept plain
/// and consistent with existing LOG style (a leading glyph, then prose).
fn format_message(kind: NotifyKind, pane: &str, detail: &str) -> String {
    match kind {
        NotifyKind::AgentDone => format!("✓ {detail} finished in {pane}"),
        NotifyKind::Failed => format!("✗ {detail} failed in {pane}"),
        NotifyKind::Bell => format!("● bell in {pane}"),
        NotifyKind::Pattern => format!("⚑ matched \"{detail}\" in {pane}"),
        NotifyKind::Exited => format!("⊗ {pane} exited"),
        NotifyKind::Waiting => format!("⧗ {pane} is waiting for you"),
        // The program wrote the words; the pane says where they came from.
        NotifyKind::Requested => format!("\u{25b8} {detail} \u{2014} {pane}"),
    }
}

#[cfg(test)]
#[path = "notify_tests.rs"]
mod tests;
