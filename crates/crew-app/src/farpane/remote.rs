//! Remote (rclone) operations for Far panels: spawn an `rclone` worker, track
//! the single in-flight op in `FarPane::pending`, and land its result in the
//! per-tick `poll_ops` — the same worker + mpsc + poll shape as `ask.rs`, so
//! the winit thread never blocks on the network.
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::SystemTime;

use super::keys::FarAction;
use super::location::Location;
use super::rclone::{self, RcloneDone};
use super::{FarPane, Side};

/// A downloaded remote file being watched for local edits to push back
/// (Task 11 uploads on change; this task only registers the mapping).
pub(crate) struct Watch {
    #[allow(dead_code)]
    pub temp: PathBuf,
    #[allow(dead_code)]
    pub remote: Location,
    #[allow(dead_code)]
    pub mtime: Option<SystemTime>,
}

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
    /// A copy/move (F5/F6) touching at least one remote endpoint (Task 9) —
    /// re-lists both panels on success (see `absorb_transfer`).
    Transfer {
        verb: &'static str,
        #[allow(dead_code)]
        refresh_both: bool,
    },
    /// F3/F4/Enter on a remote file (Task 10): `rclone copyto` the file to a
    /// local temp path, then open it and register a `Watch` for Task 11.
    Download {
        remote: Location,
        temp: PathBuf,
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
            PendingKind::Transfer { verb, .. } => {
                Some(FarAction::Status(self.absorb_transfer(verb, done)))
            }
            PendingKind::Download { remote, temp } => {
                Some(self.absorb_download(remote, temp, done))
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

    /// Kick off a copy/move (F5/F6) that touches at least one remote
    /// endpoint — `argv` is `rclone`'s `copy(to)`/`move(to)` invocation built
    /// by `fileops::copy`/`rename_move`. Re-lists both panels on success (see
    /// `absorb_transfer`).
    pub(crate) fn begin_transfer(
        &mut self,
        argv: Vec<String>,
        verb: &'static str,
        note: String,
    ) -> FarAction {
        if self.pending.is_some() {
            return FarAction::Status("rclone busy — wait for it".into());
        }
        let rx = rclone::run(argv);
        self.pending = Some(PendingOp {
            kind: PendingKind::Transfer {
                verb,
                refresh_both: true,
            },
            rx,
            note: note.clone(),
        });
        FarAction::Status(note)
    }

    /// Land a finished `Transfer` op: surface the error, or re-list both
    /// panels to reflect the change. Only one remote listing can be pending
    /// at a time, so when both sides are remote only the first (in
    /// `[Left, Right]` order) is re-listed this tick — the other is left
    /// stale until focused (accepted v1). Split out for tests, like
    /// `absorb_simple`.
    pub(crate) fn absorb_transfer(&mut self, verb: &'static str, done: RcloneDone) -> String {
        if done.code != Some(0) {
            return format!(
                "rclone: {verb} failed: {}",
                if done.stderr_tail.is_empty() {
                    "error".into()
                } else {
                    done.stderr_tail
                }
            );
        }
        // Re-list whichever sides are remote; local sides reload synchronously.
        let mut remote_listed = false;
        for side in [Side::Left, Side::Right] {
            if self.panel(side).loc.is_remote() {
                if !remote_listed {
                    let _ = self.begin_list(side);
                    remote_listed = true;
                }
            } else {
                self.panel_mut(side).reload();
            }
        }
        format!("{verb} \u{2713}")
    }

    /// F3/F4/Enter on a remote file: `rclone copyto` it to a local temp path
    /// under `<tmp>/far-drive/<name>`, so the OS-default app opens a real
    /// local file. `entry_name` is the selected file's name in the active
    /// panel; `remote` is that panel's location joined with it.
    pub(crate) fn begin_download(&mut self, entry_name: &str) -> FarAction {
        if self.pending.is_some() {
            return FarAction::Status("rclone busy — wait for it".into());
        }
        let remote = self.panel(self.active).loc.child(entry_name);
        let dir = std::env::temp_dir().join("far-drive");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return FarAction::Status(format!("download: {e}"));
        }
        let temp = dir.join(entry_name);
        let note = format!("downloading {}", remote.rclone_addr());
        let argv = rclone::argv_copy(&remote, &Location::local(&temp), false);
        let rx = rclone::run(argv);
        self.pending = Some(PendingOp {
            kind: PendingKind::Download { remote, temp },
            rx,
            note: note.clone(),
        });
        FarAction::Status(note)
    }

    /// Land a finished `Download`: surface the error, or open the temp file
    /// and register a `Watch` (temp → remote) so a future save can push the
    /// edit back (Task 11). Split out for tests, like `absorb_list`.
    pub(crate) fn absorb_download(
        &mut self,
        remote: Location,
        temp: PathBuf,
        done: RcloneDone,
    ) -> FarAction {
        if done.code != Some(0) {
            return FarAction::Status(format!(
                "rclone: download failed: {}",
                if done.stderr_tail.is_empty() {
                    "error".into()
                } else {
                    done.stderr_tail
                }
            ));
        }
        let mtime = std::fs::metadata(&temp).and_then(|m| m.modified()).ok();
        self.watches.push(Watch {
            temp: temp.clone(),
            remote,
            mtime,
        });
        FarAction::Open(temp)
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
