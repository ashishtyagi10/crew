//! `/edit <file>`: open a file in the user's terminal editor (`$VISUAL`, else
//! `$EDITOR`, else `vi`) in its own tiled pane, resolving a relative path
//! against Crew's working directory. Complements `/open`, which hands a path to
//! the OS default app instead.
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
    /// Open `arg` in the user's editor in a new pane (`/edit <file>`).
    pub(crate) fn edit_in_pane(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            self.set_status("usage: /edit <file>");
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
}

#[cfg(test)]
mod tests {
    use super::{edit_script, pick_editor, sh_quote};

    #[test]
    fn pick_editor_prefers_visual_then_editor_then_vi() {
        assert_eq!(
            pick_editor(Some("nvim".into()), Some("nano".into())),
            "nvim"
        );
        assert_eq!(pick_editor(None, Some("nano".into())), "nano");
        // blank values are ignored, falling through to the default.
        assert_eq!(pick_editor(Some("  ".into()), None), "vi");
        assert_eq!(pick_editor(None, None), "vi");
    }

    #[test]
    fn sh_quote_escapes_spaces_and_quotes() {
        assert_eq!(sh_quote("a b.txt"), "'a b.txt'");
        assert_eq!(sh_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn edit_script_quotes_path_and_keeps_pane_open() {
        let s = edit_script("code -w", "/tmp/a b.rs", "/bin/zsh");
        assert_eq!(s, "code -w '/tmp/a b.rs'; exec /bin/zsh");
    }
}
