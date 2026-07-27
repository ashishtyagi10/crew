//! `/md <path>`: opens a file as a zoomed source|preview markdown pane.
use std::path::{Path, PathBuf};

use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::mdpane::MdPane;
use crate::pane::{Pane, PaneContent};
use crate::spawn::PLACEHOLDER_RECT;

/// Resolves `arg` against `cwd`: absolute paths are kept as-is, relative
/// paths are joined onto `cwd` (Crew's tracked working directory, the same
/// one new shells start in).
fn resolve_md_path(cwd: &Path, arg: &str) -> PathBuf {
    let p = Path::new(arg);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}

impl CrewApp {
    /// Open `path` as a markdown viewer pane, zoomed full width/height, and
    /// focus it. An empty path is a usage hint; an unreadable file (missing,
    /// permissions, non-UTF-8) reports why in the status bar instead of
    /// opening an empty pane.
    pub(crate) fn spawn_md_pane(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() {
            self.set_status("usage: /md <path>");
            return;
        }
        let resolved = resolve_md_path(&self.cwd, path);
        let source = match std::fs::read_to_string(&resolved) {
            Ok(s) => s,
            Err(e) => {
                self.set_status(format!("md: cannot read {path}: {e}"));
                return;
            }
        };
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            content: PaneContent::Markdown(MdPane::new(resolved, source)),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
        });
        self.focus_new_pane();
        self.zoomed = true;
        self.redraw();
    }
}

impl CrewApp {
    /// `/about` — open the changelog that shipped with this binary, in the
    /// markdown viewer, newest release first.
    ///
    /// It used to flash "crew v0.6.62" on the status line, which answers a
    /// question nobody asks: the version number matters only as a way to
    /// find out what changed. The file is compiled in
    /// (`appregister::CHANGELOG`), so this works from an installed binary
    /// with no source tree anywhere near it — and the release that produced
    /// the binary is guaranteed to be its top entry, because the build fails
    /// otherwise.
    pub(crate) fn spawn_about_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let heading =
            crate::appregister::newest_changelog_version().unwrap_or(crate::appregister::VERSION);
        self.panes.push(Pane {
            content: PaneContent::Markdown(MdPane::new(
                PathBuf::from(format!("crew {heading}")),
                crate::appregister::CHANGELOG.to_string(),
            )),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
        });
        self.focus_new_pane();
        self.zoomed = true;
        self.redraw();
    }
}

#[cfg(test)]
#[path = "spawnmd_tests.rs"]
mod tests;
