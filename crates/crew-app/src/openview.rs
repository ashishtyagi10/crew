//! `/view <path>` (alias `/md`): opens a file in the read-only file-viewer
//! pane (`viewpane::ViewPane`) — code, markdown, CSV, diffs, and more, one
//! rung per format. See `viewpane::detect` for the format ladder.
use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::pane::{Pane, PaneContent};
use crate::spawn::PLACEHOLDER_RECT;
use crate::viewpane::ViewPane;

impl CrewApp {
    /// Open `path` in the viewer, zoomed and focused. An empty path is a
    /// usage hint; a path that does not resolve to a file reports why in the
    /// status bar rather than opening an empty pane.
    ///
    /// The `is_file` check is the one filesystem call this makes on the winit
    /// thread — the same one `clickopen::open_path_token` already makes. Every
    /// byte after it is read on a worker (`viewpane::load`).
    pub(crate) fn open_view(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() {
            self.set_status("usage: /view <path>");
            return;
        }
        let resolved = crate::pathexpand::expand_path(&self.cwd, path);
        if !resolved.is_file() {
            self.set_status(format!("view: not a file: {path}"));
            return;
        }
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            content: PaneContent::View(ViewPane::open(resolved)),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        });
        self.focus_new_pane();
        self.zoomed = true;
        self.redraw();
    }

    /// Mark the viewer `open_view` just pushed as ephemeral (Fix 4): opened
    /// on a SYNTHETIC temp file rather than something the user asked to
    /// view, so `session_panes`/`had_restorable` should act as if it were
    /// never there. Shared by `spawn_about_pane` and
    /// `askbar::absorb_explain_result`, the only two callers that write a
    /// temp file before handing it to `open_view`.
    ///
    /// `before` is `self.panes.len()` captured right before the `open_view`
    /// call: `open_view` can fail without pushing a pane (a race between the
    /// write and its own `is_file` check, astronomically unlikely but not
    /// impossible), and without this guard that would mark whatever pane
    /// happened to be LAST — a real, user-opened viewer — ephemeral instead.
    pub(crate) fn mark_last_view_ephemeral(&mut self, before: usize) {
        if self.panes.len() <= before {
            return;
        }
        if let Some(Pane {
            content: PaneContent::View(v),
            ..
        }) = self.panes.last_mut()
        {
            v.ephemeral = true;
        }
    }
}

impl CrewApp {
    /// `/about` — open the changelog that shipped with this binary, in the
    /// file viewer, newest release first.
    ///
    /// It used to flash "crew v0.6.62" on the status line, which answers a
    /// question nobody asks: the version number matters only as a way to
    /// find out what changed. The file is compiled in
    /// (`appregister::CHANGELOG`), so this works from an installed binary
    /// with no source tree anywhere near it — and the release that produced
    /// the binary is guaranteed to be its top entry, because the build fails
    /// otherwise.
    ///
    /// `ViewPane::open` only reads real files off disk, so the compiled-in
    /// text is written to a temp path first (named after the release it
    /// documents, so re-running `/about` on the same build overwrites rather
    /// than litters) and handed to `open_view`. That write is the same kind
    /// of one-shot synchronous I/O every other pane spawn on this thread
    /// already does.
    pub(crate) fn spawn_about_pane(&mut self) {
        let heading =
            crate::appregister::newest_changelog_version().unwrap_or(crate::appregister::VERSION);
        let path = std::env::temp_dir().join(format!("crew-changelog-{heading}.md"));
        if let Err(e) = std::fs::write(&path, crate::appregister::CHANGELOG) {
            self.set_status(format!("about: cannot open changelog: {e}"));
            return;
        }
        let before = self.panes.len();
        self.open_view(&path.to_string_lossy());
        self.mark_last_view_ephemeral(before);
    }
}

#[cfg(test)]
#[path = "openview_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "viewpane/open_tests.rs"]
mod open_tests;

impl CrewApp {
    /// `/log` — this session's full activity trail in the file viewer (the
    /// sidebar LOG shows only a 5-line tail). `r` in the viewer re-reads,
    /// so it doubles as a poor man's tail -f.
    pub(crate) fn open_log(&mut self) {
        match crate::activitylog::path().filter(|p| p.is_file()) {
            Some(p) => self.open_view(&p.to_string_lossy()),
            None => self.set_status("no activity logged yet this session"),
        }
    }
}
