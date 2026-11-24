//! `/about`: the changelog compiled into this binary, in the file viewer.
use crate::app::CrewApp;

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
        self.name_last_view(&format!("what's new \u{b7} {heading}"));
        self.mark_last_view_ephemeral(before);
    }
}
