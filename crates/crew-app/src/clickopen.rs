//! Cmd/Ctrl+click resolution: in terminal panes, open a URL, edit a file in
//! `$EDITOR`, or `cd` into a directory — whichever the clicked text resolves
//! to (builds on `openurl` and reuses `/edit` and `cd`); in chat panes, open a
//! markdown link's URL (`chatview::link_at`).
use crate::app::CrewApp;
use crate::openurl::url_at;
use crate::pane::PaneContent;

/// The whitespace-delimited token spanning character column `col` in `line`,
/// stripped of surrounding quotes/brackets/punctuation. `None` over whitespace.
pub(crate) fn token_at(line: &str, col: usize) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    if col >= chars.len() || chars[col].is_whitespace() {
        return None;
    }
    let mut a = col;
    while a > 0 && !chars[a - 1].is_whitespace() {
        a -= 1;
    }
    let mut b = col;
    while b < chars.len() && !chars[b].is_whitespace() {
        b += 1;
    }
    let trim = |c: char| "\"'()[]{}<>,:;".contains(c);
    while a < b && trim(chars[a]) {
        a += 1;
    }
    while b > a && trim(chars[b - 1]) {
        b -= 1;
    }
    (a < b).then(|| chars[a..b].iter().collect())
}

impl CrewApp {
    /// Resolve a Cmd/Ctrl+click under the cursor: in a terminal pane, a URL
    /// opens in the browser, an existing file opens in `$EDITOR`, a directory
    /// becomes the new cwd; in a chat pane, a markdown link opens its URL.
    /// Returns `true` when it acted on something (a miss falls through to the
    /// caller's normal click handling — selection/focus).
    pub(crate) fn cmd_click_at_cursor(&mut self) -> bool {
        if let Some((line, col)) = self.cursor_cell() {
            if let Some(url) = url_at(&line, col) {
                let _ = open::that(&url);
                self.set_status(format!("opening {url}"));
                return true;
            }
            return match token_at(&line, col) {
                Some(tok) => self.open_path_token(&tok),
                None => false,
            };
        }
        self.chat_link_click_at_cursor()
    }

    /// Chat-pane counterpart of the terminal miss above: if the cursor sits
    /// over a Chat pane's rendered markdown link, open it.
    fn chat_link_click_at_cursor(&mut self) -> bool {
        let Some(i) = self.pane_at_cursor() else {
            return false;
        };
        let Some((row, col)) = self.cursor_rowcol(i) else {
            return false;
        };
        let pane = &self.panes[i];
        let PaneContent::Chat(chat) = &pane.content else {
            return false;
        };
        let Some(url) =
            crate::chatview::link_at(chat, pane.grid.cols, pane.grid.rows, row as u16, col as u16)
        else {
            return false;
        };
        let _ = open::that(&url);
        self.set_status(format!("opening {url}"));
        true
    }

    /// If `tok` resolves (against the cwd) to a file, edit it; to a directory, cd.
    fn open_path_token(&mut self, tok: &str) -> bool {
        let base = if self.cwd.as_os_str().is_empty() {
            std::path::PathBuf::from(".")
        } else {
            self.cwd.clone()
        };
        let p = std::path::Path::new(tok);
        let full = if p.is_absolute() {
            p.to_path_buf()
        } else {
            base.join(p)
        };
        if full.is_file() {
            self.edit_in_pane(tok);
            true
        } else if full.is_dir() {
            self.try_change_dir(&format!("cd {tok}"))
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::token_at;

    #[test]
    fn token_at_extracts_word_and_trims_punctuation() {
        let line = "edit src/main.rs, please";
        let i = line.find("src").unwrap();
        assert_eq!(token_at(line, i + 1).as_deref(), Some("src/main.rs"));
        // surrounding quotes/parens are stripped.
        assert_eq!(token_at("(foo/bar)", 2).as_deref(), Some("foo/bar"));
        assert_eq!(token_at("\"a/b\"", 2).as_deref(), Some("a/b"));
    }

    #[test]
    fn token_at_over_whitespace_is_none() {
        assert_eq!(token_at("a b", 1), None);
        assert_eq!(token_at("word", 99), None);
        // a token that is only punctuation trims to nothing.
        assert_eq!(token_at("(),", 0), None);
    }
}
