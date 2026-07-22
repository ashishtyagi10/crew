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
    List {
        side: Side,
        loc: Location,
    },
    /// `rclone listremotes`, landing into the drive-select overlay.
    Remotes,
    /// A generic "run this rclone op, then re-list the affected panel" op —
    /// delete (Task 7), mkdir (Task 8), and further mutations reuse this.
    Simple {
        refresh: Side,
        verb: &'static str,
    },
}

pub(crate) struct PendingOp {
    pub kind: PendingKind,
    pub rx: Receiver<RcloneDone>,
    /// Short label shown while it runs (e.g. "listing gdrive:Photos"); wired
    /// into the render loading state by a later task.
    #[allow(dead_code)]
    pub note: String,
}

/// One selectable row in the drive-select overlay (Alt+F1/F2).
pub(crate) enum DriveOption {
    Local,
    Remote(String), // remote name without the trailing ':'
}

/// The Alt+F1/F2 drive-select overlay: which panel it targets, the choices
/// (empty while `listremotes` is still running), and the highlighted row.
pub(crate) struct DriveSelect {
    pub side: Side,
    pub options: Vec<DriveOption>,
    pub sel: usize,
}

impl DriveSelect {
    /// The overlay shown while `listremotes` is still running.
    pub(crate) fn loading(side: Side) -> Self {
        Self {
            side,
            options: Vec::new(),
            sel: 0,
        }
    }
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
            PendingKind::Remotes => Some(FarAction::Status(self.absorb_remotes(done))),
            PendingKind::Simple { refresh, verb } => {
                Some(FarAction::Status(self.absorb_simple(refresh, verb, done)))
            }
        }
    }

    /// Kick off a generic rclone mutation (delete, mkdir, ...) that re-lists
    /// `refresh`'s panel on success. `verb` labels the status line (e.g.
    /// "deleted").
    pub(crate) fn begin_simple(
        &mut self,
        argv: Vec<String>,
        refresh: Side,
        verb: &'static str,
        note: String,
    ) -> FarAction {
        if self.pending.is_some() {
            return FarAction::Status("rclone busy — wait for it".into());
        }
        let rx = rclone::run(argv);
        self.pending = Some(PendingOp {
            kind: PendingKind::Simple { refresh, verb },
            rx,
            note: note.clone(),
        });
        FarAction::Status(note)
    }

    /// Land a finished `Simple` op: surface the error, or re-list `refresh`'s
    /// panel to reflect the change. Split out for tests, like `absorb_list`.
    pub(crate) fn absorb_simple(
        &mut self,
        refresh: Side,
        verb: &'static str,
        done: RcloneDone,
    ) -> String {
        if done.code != Some(0) {
            return format!(
                "rclone: {} failed: {}",
                verb,
                if done.stderr_tail.is_empty() {
                    "error".into()
                } else {
                    done.stderr_tail
                }
            );
        }
        // Re-list the affected panel to reflect the change.
        let _ = self.begin_list(refresh);
        format!("{verb} \u{2713}")
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

    /// Alt+F1 (left) / Alt+F2 (right): open the drive-select overlay for
    /// `side` and kick off `rclone listremotes` in the background. Refuses
    /// (with a status line, no overlay) when `rclone` isn't installed or
    /// another remote op is already in flight.
    pub(crate) fn open_drive_select(&mut self, side: Side) -> FarAction {
        if !rclone::available() {
            return FarAction::Status(
                "rclone not found — install it and run `rclone config`".into(),
            );
        }
        if self.pending.is_some() {
            return FarAction::Status("rclone busy — wait for it".into());
        }
        self.drive_select = Some(DriveSelect::loading(side));
        let rx = rclone::run(rclone::argv_listremotes());
        self.pending = Some(PendingOp {
            kind: PendingKind::Remotes,
            rx,
            note: "listing remotes".into(),
        });
        FarAction::Status("choose a drive\u{2026}".into())
    }

    /// Land a finished `listremotes`: populate the overlay's options (`Local`
    /// plus one `Remote` per non-blank output line) or close it and surface
    /// the error. Split out for tests, like `absorb_list`.
    pub(crate) fn absorb_remotes(&mut self, done: RcloneDone) -> String {
        let Some(ds) = self.drive_select.as_mut() else {
            return String::new();
        };
        if done.code != Some(0) {
            self.drive_select = None;
            return format!(
                "rclone: {}",
                if done.stderr_tail.is_empty() {
                    "listremotes failed".to_string()
                } else {
                    done.stderr_tail
                }
            );
        }
        let mut options = vec![DriveOption::Local];
        for line in done.stdout.lines().map(str::trim).filter(|l| !l.is_empty()) {
            options.push(DriveOption::Remote(line.trim_end_matches(':').to_string()));
        }
        ds.options = options;
        ds.sel = 0;
        "choose a drive \u{2014} Enter to open, Esc to cancel".into()
    }

    /// Apply the highlighted overlay option: re-root `side`'s panel to the
    /// local cwd (`Local`) or to a remote's root, kicking off its listing
    /// (`Remote`). Closes the overlay either way.
    pub(crate) fn choose_drive(&mut self) -> Option<FarAction> {
        let ds = self.drive_select.take()?;
        let option = ds.options.into_iter().nth(ds.sel)?;
        let side = ds.side;
        match option {
            DriveOption::Local => {
                // Re-root to the process cwd (or temp as a last resort).
                let cwd = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
                let panel = self.panel_mut(side);
                panel.loc = Location::local(&cwd);
                panel.sel = 0;
                panel.reload();
                Some(FarAction::Status(format!(
                    "local \u{2014} {}",
                    cwd.display()
                )))
            }
            DriveOption::Remote(remote) => {
                let panel = self.panel_mut(side);
                panel.loc = Location {
                    backend: super::location::Backend::Rclone { remote },
                    path: String::new(),
                };
                panel.sel = 0;
                panel.entries.clear();
                Some(self.begin_list(side))
            }
        }
    }
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
