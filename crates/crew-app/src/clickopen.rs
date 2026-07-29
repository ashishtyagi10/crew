//! Cmd/Ctrl+click resolution: in terminal panes, open a URL, edit a file in
//! `$EDITOR`, or `cd` into a directory — whichever the clicked text resolves
//! to (builds on `openurl` and reuses `/edit` and `cd`); in chat panes, open a
//! markdown link's URL (`chatview::link_at`); in the file-viewer pane, open a
//! link on its rendered markdown rung, read off the cell under the click.
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

/// The link URL under pane-relative `(row, col)` in the viewer's rendered
/// `lines`, given the pane's current `scroll` offset. Mirrors the top-anchored
/// windowing `ViewPane::cells` draws with (`viewpane::render`) — `scroll`
/// clamped to the last line, then `row` counted down from there — so a click
/// always resolves the line it visibly sits on. Pulled out of the click
/// handler as a pure function so the row/col → cell mapping is testable
/// without a live `ViewPane` or `CrewApp`.
fn view_link_at(
    lines: &[crate::chatbody::CardLine],
    scroll: usize,
    row: u16,
    col: u16,
) -> Option<String> {
    let top = scroll.min(lines.len().saturating_sub(1));
    let idx = top + row as usize;
    lines
        .get(idx)
        .and_then(|line| crate::chatplace::cell_at_col(line, col))
        .and_then(|cell| cell.link.as_deref())
        .map(str::to_string)
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
    /// over a Chat pane's rendered markdown link, open it. Falls through to
    /// the file-viewer pane case if not Chat.
    fn chat_link_click_at_cursor(&mut self) -> bool {
        let Some(i) = self.pane_at_cursor() else {
            return false;
        };
        let Some((row, col)) = self.cursor_rowcol(i) else {
            return false;
        };
        let pane = &self.panes[i];
        match &pane.content {
            PaneContent::Chat(chat) => {
                if let Some(url) = crate::chatview::link_at(
                    chat,
                    pane.grid.cols,
                    pane.grid.rows,
                    row as u16,
                    col as u16,
                ) {
                    let _ = open::that(&url);
                    self.set_status(format!("opening {url}"));
                    return true;
                }
                // Not a link: a code block is the other thing worth acting on.
                let Some(code) = crate::chatview::code_block_at(
                    chat,
                    pane.grid.cols,
                    pane.grid.rows,
                    row as u16,
                ) else {
                    return false;
                };
                let lines = code.lines().count();
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_text(code.clone());
                }
                self.set_status(format!(
                    "copied {lines} line{}",
                    if lines == 1 { "" } else { "s" }
                ));
                true
            }
            PaneContent::View(v) => {
                let url = {
                    let cache = v.lines_for(pane.grid.cols);
                    view_link_at(&cache.lines, v.scroll, row as u16, col as u16)
                };
                let Some(url) = url else {
                    return false;
                };
                let _ = open::that(&url);
                self.set_status(format!("opening {url}"));
                true
            }
            _ => false,
        }
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
    use super::{token_at, view_link_at};
    use crate::chatbody::{plain, CardCell, CardLine};

    /// A one-line fixture whose whole line is a link (mirrors how
    /// `mdrung::lines`/`chatmd` tag a link span's cells).
    fn link_line(text: &str, url: &str) -> CardLine {
        text.chars()
            .map(|c| CardCell {
                link: Some(std::sync::Arc::from(url)),
                ..plain(c, (0, 0, 0), false)
            })
            .collect()
    }

    #[test]
    fn view_link_at_hits_a_link_cell_and_misses_plain_text() {
        let lines = vec![
            vec![plain('p', (0, 0, 0), false), plain('l', (0, 0, 0), false)],
            link_line("go", "https://x.io"),
        ];
        assert_eq!(
            view_link_at(&lines, 0, 1, 0),
            Some("https://x.io".to_string()),
            "the link line resolves"
        );
        assert_eq!(
            view_link_at(&lines, 0, 0, 0),
            None,
            "plain text is never a link"
        );
    }

    #[test]
    fn view_link_at_respects_the_scroll_offset() {
        // Three lines, scrolled down by 1: pane-relative row 0 is source
        // line 1 (the link), not line 0.
        let lines = vec![
            vec![plain('a', (0, 0, 0), false)],
            link_line("b", "https://s.io"),
            vec![plain('c', (0, 0, 0), false)],
        ];
        assert_eq!(
            view_link_at(&lines, 1, 0, 0),
            Some("https://s.io".to_string())
        );
        // Without the scroll, row 0 is the plain first line — no link.
        assert_eq!(view_link_at(&lines, 0, 0, 0), None);
    }

    #[test]
    fn view_link_at_clamps_a_stale_scroll_to_match_what_cells_draws() {
        // If `scroll` is momentarily stale (larger than the content — e.g.
        // right after a reload shrank the file, before `clamp_scroll` has
        // run again), `ViewPane::cells` still draws SOMETHING at row 0: the
        // last line, per its own `top = scroll.min(len - 1)` clamp
        // (`viewpane::render`). This must resolve the same line, or a click
        // could silently miss content that is visibly on screen.
        let lines = vec![
            vec![plain('a', (0, 0, 0), false)],
            link_line("b", "https://last.io"),
        ];
        assert_eq!(
            view_link_at(&lines, 999, 0, 0),
            Some("https://last.io".to_string()),
            "an over-large scroll still resolves the line actually drawn at row 0"
        );
    }

    #[test]
    fn view_link_at_out_of_bounds_is_none_not_a_panic() {
        let lines = vec![link_line("x", "https://x.io")];
        assert_eq!(view_link_at(&lines, 0, 50, 0), None, "row past the end");
        assert_eq!(view_link_at(&lines, 0, 0, 50), None, "col past the end");
        assert_eq!(view_link_at(&[], 0, 0, 0), None, "no lines at all");
    }

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
