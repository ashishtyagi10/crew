//! Transient status messages flashed on the input card's bottom border (e.g.
//! "copied 12 lines", "cd: no such directory"), auto-expiring after a few
//! seconds so the bar normally stays clean.
use std::time::{Duration, Instant};

use crate::app::CrewApp;

/// How long a status message stays visible.
const STATUS_TTL: Duration = Duration::from_secs(3);

/// Most entries kept in the live LOG ring buffer (oldest dropped past this).
pub(crate) const LOG_CAP: usize = 64;

impl CrewApp {
    /// Flash a transient status message and request a redraw. The message is also
    /// appended to the live LOG ring buffer shown in the left nav — unlike the
    /// 3-second flash, the log keeps a scrollback of recent activity.
    pub(crate) fn set_status(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        if self.log.len() >= LOG_CAP {
            self.log.remove(0);
        }
        self.log.push(msg.clone());
        self.status = Some((msg, Instant::now()));
        self.redraw();
    }

    /// The current status text, or `None` once it has expired.
    pub(crate) fn active_status(&self) -> Option<&str> {
        self.status
            .as_ref()
            .filter(|(_, t)| t.elapsed() < STATUS_TTL)
            .map(|(s, _)| s.as_str())
    }

    /// Drop an expired status; returns `true` when one was cleared (so the
    /// caller knows to repaint the now-empty bottom border).
    pub(crate) fn expire_status(&mut self) -> bool {
        let expired = self
            .status
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed() >= STATUS_TTL);
        if expired {
            self.status = None;
        }
        expired
    }
}
