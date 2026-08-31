//! Taking in what a finished rclone operation produced: a listing, a
//! transfer, a download, the remote roster.
//!
//! Split from [`super::remote`] for the line cap, along the line between
//! STARTING an operation and taking in what it returned — the two halves the
//! worker-thread boundary already divided the file into.
use super::keys::FarAction;
use super::location::Location;
use super::rclone::{self, RcloneDone};
use super::remote::{DriveOption, Watch};
use super::{FarPane, Side};
use std::path::PathBuf;

impl FarPane {
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
        // Dedupe by temp path: re-downloading the same remote address (which
        // — per `download_temp` — always resolves to the same temp) updates
        // the existing `Watch` in place instead of pushing a duplicate. With
        // unique-per-address temps from part 1, two DIFFERENT remotes never
        // share a temp, so this can only ever collapse re-downloads of the
        // SAME file — it never merges two distinct files' watches.
        if let Some(w) = self.watches.iter_mut().find(|w| w.temp == temp) {
            w.remote = remote;
            w.mtime = mtime;
        } else {
            self.watches.push(Watch {
                temp: temp.clone(),
                remote,
                mtime,
            });
        }
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
}
