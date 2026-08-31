//! Which of the `/far` pane's two panels is active, what it is looking at,
//! and reaching the other one.
//!
//! Split out of [`super`] for the line cap.
use super::location::Location;
use super::{FarPane, Panel, Side};
use std::path::PathBuf;

impl FarPane {
    /// The active panel's location.
    pub(crate) fn active_loc(&self) -> Location {
        self.panel(self.active).loc.clone()
    }

    /// The active panel's directory as a local path — the working dir for the
    /// bottom command line, which is LOCAL-ONLY in v1. A remote active panel
    /// yields the temp dir as an inert fallback (the command line is disabled
    /// for remote panels in `run.rs`).
    pub(crate) fn active_cwd(&self) -> PathBuf {
        self.active_loc()
            .local_path()
            .unwrap_or_else(std::env::temp_dir)
    }

    /// The active panel's directory label for the command bar: the last path
    /// segment (or the whole string when there's no segment, e.g. at a root).
    /// For a LOCAL panel this uses `Path::file_name()` exactly as before the
    /// `Location` refactor, so a trailing separator (e.g. from `cd sub/`) is
    /// insignificant — `/tmp/sub/` shows `sub`, not the full path. Remote
    /// panels have no `Path` to lean on, so they derive the label from
    /// `Location::display`, trimming a trailing `/` first for the same
    /// trailing-separator insensitivity (`gdrive:Photos/` shows `Photos`).
    pub(crate) fn active_panel_folder(&self) -> String {
        let loc = self.active_loc();
        if let Some(path) = loc.local_path() {
            return path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| loc.display());
        }
        let display = loc.display();
        display
            .trim_end_matches(['/', '\\'])
            .rsplit(['/', '\\'])
            .next()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or(display)
    }

    pub(crate) fn active_panel_mut(&mut self) -> &mut Panel {
        self.panel_mut(self.active)
    }

    /// The panel on the side *opposite* the active one — the destination for
    /// copy/move operations.
    pub(crate) fn other_side(&self) -> Side {
        match self.active {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub(crate) fn panel(&self, side: Side) -> &Panel {
        match side {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }

    pub(crate) fn panel_mut(&mut self, side: Side) -> &mut Panel {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    /// Re-read both panels after a filesystem change so each side reflects it
    /// (the two panels often show the same directory).
    pub(crate) fn reload_both(&mut self) {
        self.left.reload();
        self.right.reload();
    }
}
