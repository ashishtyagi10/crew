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
    /// A program rang the terminal bell.
    Bell,
    /// A watched substring appeared in the pane's output.
    Pattern,
    /// The pane's process exited.
    Exited,
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
            let long_enough = since.is_some_and(|s| now.saturating_duration_since(s) >= min);
            AgentDone {
                finished: long_enough.then(|| cmd.to_string()),
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
        NotifyKind::Bell => format!("● bell in {pane}"),
        NotifyKind::Pattern => format!("⚑ matched \"{detail}\" in {pane}"),
        NotifyKind::Exited => format!("⊗ {pane} exited"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_returns_a_message_naming_the_command_and_pane() {
        let mut n = Notifier::default();
        let msg = n
            .record(
                NotifyKind::AgentDone,
                "crew".into(),
                "claude".into(),
                Instant::now(),
            )
            .expect("a fresh event surfaces");
        assert!(msg.contains("claude"));
        assert!(msg.contains("crew"));
        assert_eq!(n.len(), 1);
    }

    #[test]
    fn identical_event_within_cooldown_is_throttled() {
        let mut n = Notifier::default();
        let t0 = Instant::now();
        assert!(n
            .record(NotifyKind::Bell, "a".into(), String::new(), t0)
            .is_some());
        // Same kind+pane+detail 5s later → suppressed.
        let later = t0 + Duration::from_secs(5);
        assert!(n
            .record(NotifyKind::Bell, "a".into(), String::new(), later)
            .is_none());
    }

    #[test]
    fn same_event_surfaces_again_after_cooldown() {
        let mut n = Notifier::default();
        let t0 = Instant::now();
        assert!(n
            .record(NotifyKind::Bell, "a".into(), String::new(), t0)
            .is_some());
        let after = t0 + Duration::from_secs(11);
        assert!(n
            .record(NotifyKind::Bell, "a".into(), String::new(), after)
            .is_some());
    }

    #[test]
    fn different_pane_is_not_throttled() {
        let mut n = Notifier::default();
        let t0 = Instant::now();
        assert!(n
            .record(NotifyKind::Bell, "a".into(), String::new(), t0)
            .is_some());
        assert!(n
            .record(NotifyKind::Bell, "b".into(), String::new(), t0)
            .is_some());
    }

    #[test]
    fn recent_ring_is_capped() {
        let mut n = Notifier::default();
        let t0 = Instant::now();
        for i in 0..(CAP + 10) {
            // Distinct detail each time so none are throttled.
            n.record(NotifyKind::Pattern, "p".into(), i.to_string(), t0);
        }
        assert_eq!(n.len(), CAP);
    }

    #[test]
    fn each_kind_formats_distinctly() {
        assert!(format_message(NotifyKind::AgentDone, "p", "claude").contains("finished"));
        assert!(format_message(NotifyKind::Bell, "p", "").contains("bell"));
        assert!(format_message(NotifyKind::Pattern, "p", "error").contains("error"));
        assert!(format_message(NotifyKind::Exited, "p", "").contains("exited"));
    }

    #[test]
    fn agent_done_fires_after_the_threshold() {
        let t0 = Instant::now();
        let out = agent_done(
            Some("claude"),
            None,
            Some(t0),
            Duration::from_secs(10),
            t0 + Duration::from_secs(11),
        );
        assert_eq!(out.finished.as_deref(), Some("claude"));
        assert_eq!(out.since, None);
    }

    #[test]
    fn agent_done_suppressed_under_the_threshold() {
        let t0 = Instant::now();
        let out = agent_done(
            Some("ls"),
            None,
            Some(t0),
            Duration::from_secs(10),
            t0 + Duration::from_secs(3),
        );
        assert_eq!(out.finished, None);
        assert_eq!(out.since, None);
    }

    #[test]
    fn agent_done_starts_the_timer_on_launch() {
        let t0 = Instant::now();
        let out = agent_done(None, Some("claude"), None, Duration::from_secs(10), t0);
        assert_eq!(out.finished, None);
        assert_eq!(out.since, Some(t0));
    }

    #[test]
    fn agent_done_ignores_command_to_command_changes() {
        let t0 = Instant::now();
        let out = agent_done(
            Some("cargo"),
            Some("rustc"),
            Some(t0),
            Duration::from_secs(10),
            t0 + Duration::from_secs(11),
        );
        assert_eq!(out.finished, None);
        // The original start is preserved so the whole burst is timed as one.
        assert_eq!(out.since, Some(t0));
    }

    #[test]
    fn agent_done_without_a_recorded_start_does_not_fire() {
        let t0 = Instant::now();
        let out = agent_done(Some("claude"), None, None, Duration::from_secs(10), t0);
        assert_eq!(out.finished, None);
    }
}
