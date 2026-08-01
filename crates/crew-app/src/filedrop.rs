//! Dropped-file routing (`WindowEvent::DroppedFile`): a file dragged from
//! Finder lands in the focused pane — as an `@path ` mention in a chat
//! composer, as a shell-quoted path written to a terminal's PTY. Multiple
//! files arrive as one event each; every drop appends, so they end up
//! space-separated in arrival order.
//!
//! Focused pane rather than pane-under-cursor, deliberately: winit 0.30
//! reports no drag position on any backend — macOS emits `HoveredFile` once
//! at `draggingEntered:` and never a `CursorMoved` while the OS drag session
//! suppresses mouse events (Windows' drop handler likewise discards its
//! `POINTL`) — so `self.cursor` here is stale from before the drag began,
//! and `pane_at_cursor` would silently misdeliver to whatever pane the
//! mouse last rested on.
use std::io::Write;
use std::path::Path;

use crate::app::CrewApp;
use crate::pane::PaneContent;

/// The composer token for a dropped file: `@path `, relative to `cwd` when
/// the file is inside it — matching the relative paths `fileindex::scan`
/// feeds the mention popup, so `chatmention::expand` resolves it at send —
/// and absolute otherwise. The trailing space ends the token, exactly as
/// `chatmention::accept` finishes a picked one. A path containing whitespace
/// is minted as `@"path" ` — `chatmention`'s tokenizer is whitespace-
/// delimited, so the bare form would silently drop at send.
pub(crate) fn mention_token(path: &Path, cwd: &Path) -> String {
    let rel = path
        .strip_prefix(cwd)
        .ok()
        // Dropping the cwd itself would strip to "" — "@ " mentions nothing.
        .filter(|r| !r.as_os_str().is_empty());
    let shown = rel.unwrap_or(path).display();
    let shown = shown.to_string();
    if shown.chars().any(char::is_whitespace) {
        format!("@\"{shown}\" ")
    } else {
        format!("@{shown} ")
    }
}

/// The terminal form: the absolute path single-quoted the way terminals
/// paste paths (embedded `'` becomes `'\''`), plus the same trailing space.
pub(crate) fn shell_quoted(path: &Path) -> String {
    format!("'{}' ", path.display().to_string().replace('\'', "'\\''"))
}

impl CrewApp {
    /// Route one dropped file into the focused pane. Hidden (minimized)
    /// panes and panes with no input surface ignore the drop.
    pub(crate) fn drop_file(&mut self, path: &Path) {
        // The app cwd, NOT `pane.dir`: `keys.rs` hands `chatmention::expand`
        // exactly `self.cwd` at send time, and the token minted here must
        // relativize against the SAME root — the two must stay in lockstep,
        // or a drop-time token silently fails to resolve at send.
        let cwd = self.cwd.clone();
        let Some(pane) = self.panes.get_mut(self.focused) else {
            return; // no panes at all
        };
        if pane.hidden {
            return;
        }
        let note = match &mut pane.content {
            PaneContent::Chat(c) => {
                // A drop mid-Ctrl+R would be silently undone: the search's
                // Close/Accept restore the composer from its `saved`
                // snapshot. Close it first (restoring the draft), THEN append.
                crate::chathistsearch::close_restoring(&mut c.histsearch, &mut c.input);
                let token = mention_token(path, &cwd);
                // A separating space when the composer ends mid-word: gluing
                // the token onto "summarize" (or a half-typed "@sr") makes
                // one broken token `expand` never resolves.
                if !c.input.is_empty() && !c.input.ends_with(char::is_whitespace) {
                    c.input.push(' ');
                }
                c.input.push_str(&token);
                // Same contract as a typed edit (`ChatPane::on_input`): the
                // completed token is no pending mention, so this closes any
                // popup left open mid-typing instead of leaving stale
                // matches. The scan closure never runs on a closed token.
                crate::chatmention::after_edit(&mut c.mention, &c.input, Vec::new);
                format!("dropped file \u{2192} {}", token.trim_end())
            }
            PaneContent::Terminal(t) => {
                let quoted = shell_quoted(path);
                // The paste write path (`clipboard::insert_paste`): bracketed
                // when the running program asked for it, straight bytes else.
                let bytes = crate::session::wrap_paste(&quoted, t.pty.bracketed_paste());
                t.pty.scroll_to_bottom();
                if let Err(e) = t.input.write_all(&bytes).and_then(|_| t.input.flush()) {
                    eprintln!("drop write error: {e}");
                }
                format!("dropped file \u{2192} {}", quoted.trim_end())
            }
            PaneContent::Settings(_)
            | PaneContent::Far(_)
            | PaneContent::Swarm(_)
            | PaneContent::View(_) => return,
        };
        self.set_status(note);
        self.redraw();
    }
}

#[cfg(test)]
#[path = "filedrop_tests.rs"]
mod tests;
