//! Open a file in the user's terminal editor (`$VISUAL`, else `$EDITOR`, else
//! `vi`) in its own tiled pane, resolving a relative path against Crew's
//! working directory. Reached by Cmd+clicking a file path in a terminal pane
//! (see `clickopen`); browsing/opening files by name is Far's job (`/far`).
use crate::app::CrewApp;
use crate::spawn::default_shell;

/// Pick the editor: `$VISUAL`, then `$EDITOR`, then `vi`. Pure for testing.
pub(crate) fn pick_editor(visual: Option<String>, editor: Option<String>) -> String {
    visual
        .filter(|s| !s.trim().is_empty())
        .or(editor.filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "vi".to_string())
}

/// Single-quote `s` for POSIX shells, escaping embedded quotes (so paths with
/// spaces or special characters survive `sh -c`).
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The `sh -c` script: run `editor path`, then re-exec `shell` so the pane stays
/// open (e.g. to read editor messages) after the editor exits.
pub(crate) fn edit_script(editor: &str, path: &str, shell: &str) -> String {
    format!("{editor} {}; exec {shell}", sh_quote(path))
}

impl CrewApp {
    /// Open `arg` in the user's editor in a new pane (Cmd+click on a file path).
    pub(crate) fn edit_in_pane(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            return;
        }
        let path = crate::pathexpand::expand_path(&self.cwd, arg)
            .to_string_lossy()
            .into_owned();
        let editor = pick_editor(std::env::var("VISUAL").ok(), std::env::var("EDITOR").ok());
        let shell = default_shell();
        let script = edit_script(&editor, &path, &shell);
        let label = editor
            .split_whitespace()
            .next()
            .unwrap_or("edit")
            .to_string();
        self.spawn_labeled_terminal(&shell, &["-c".to_string(), script], label);
    }

    /// The viewer's `e` key: spawn `$EDITOR` on `path`, and — only if that
    /// spawn actually produced a pane — remember its `born_ms` on the viewer
    /// at `focused` so `poll::reload_views_after_edit` can find it once the
    /// editor exits.
    ///
    /// `edit_in_pane` pushes no pane on a failed spawn (see
    /// `spawn_labeled_terminal`'s `Err` arm, which only sets a status) or on
    /// an empty path. Reading `self.panes.last()` unconditionally in either
    /// case would silently adopt whatever pane happens to be last — e.g. an
    /// unrelated already-running terminal's `born_ms` — and the viewer would
    /// then wait on that pane going idle before ever reloading again, rather
    /// than on the edit that never actually started.
    pub(crate) fn apply_view_edit(&mut self, focused: usize, path: &std::path::Path) {
        let before = self.panes.len();
        self.edit_in_pane(&path.to_string_lossy());
        if self.panes.len() == before {
            return;
        }
        let born = self.panes.last().map(|p| p.born_ms);
        if let Some(crate::pane::PaneContent::View(v)) =
            self.panes.get_mut(focused).map(|p| &mut p.content)
        {
            v.editor_born = born;
        }
    }
}

#[cfg(test)]
#[path = "editpane_tests.rs"]
mod tests;
