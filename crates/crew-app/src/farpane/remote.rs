//! Remote (rclone) operations for Far panels: spawn an `rclone` worker, track
//! the single in-flight op in `FarPane::pending`, and land its result in the
//! per-tick `poll_ops` — the same worker + mpsc + poll shape as `ask.rs`, so
//! the winit thread never blocks on the network.
use std::sync::mpsc::Receiver;

use super::keys::FarAction;
use super::location::Location;
use super::rclone::{self, RcloneDone};
use super::{FarPane, Side};

/// Which remote op a `PendingOp` represents (extended in Tasks 7-10).
pub(crate) enum PendingKind {
    List { side: Side, loc: Location },
}

pub(crate) struct PendingOp {
    pub kind: PendingKind,
    pub rx: Receiver<RcloneDone>,
    /// Short label shown while it runs (e.g. "listing gdrive:Photos"); wired
    /// into the render loading state by a later task.
    #[allow(dead_code)]
    pub note: String,
}

impl FarPane {
    /// Kick off an `rclone lsjson` for `side`'s current remote location.
    pub(crate) fn begin_list(&mut self, side: Side) -> FarAction {
        if self.pending.is_some() {
            return FarAction::Status("rclone busy — wait for it".into());
        }
        let loc = self.panel(side).loc.clone();
        if !loc.is_remote() {
            return FarAction::Status("not a remote panel".into());
        }
        self.panel_mut(side).loading = true;
        let note = format!("listing {}", loc.rclone_addr());
        let rx = rclone::run(rclone::argv_lsjson(&loc));
        self.pending = Some(PendingOp {
            kind: PendingKind::List { side, loc },
            rx,
            note: note.clone(),
        });
        FarAction::Status(note)
    }

    /// Drain a finished remote op this tick, if any.
    pub fn poll_ops(&mut self) -> Option<FarAction> {
        let pending = self.pending.as_ref()?;
        let done = pending.rx.try_recv().ok()?;
        let kind = self.pending.take().map(|p| p.kind)?;
        match kind {
            PendingKind::List { side, loc } => {
                Some(FarAction::Status(self.absorb_list(side, loc, done)))
            }
        }
    }

    /// Install a finished listing (or surface its error). Split out for tests.
    pub(crate) fn absorb_list(&mut self, side: Side, loc: Location, done: RcloneDone) -> String {
        self.panel_mut(side).loading = false;
        if done.code != Some(0) {
            return format!(
                "rclone: {}",
                if done.stderr_tail.is_empty() {
                    "listing failed".to_string()
                } else {
                    done.stderr_tail
                }
            );
        }
        match rclone::parse_lsjson(&done.stdout, &loc) {
            Ok(entries) => {
                let panel = self.panel_mut(side);
                panel.entries = entries;
                panel.sel = 0;
                format!(
                    "{} — {} items",
                    loc.rclone_addr(),
                    self.panel(side).entries.len()
                )
            }
            Err(e) => format!("rclone: bad listing: {e}"),
        }
    }

    /// Whether a remote op is in flight (feeds the busy sweep / spinner).
    pub(crate) fn ops_busy(&self) -> bool {
        self.pending.is_some()
    }
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
