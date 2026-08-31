//! Cmd/Ctrl+click resolution: in terminal panes, open a URL, show a file in
//! the viewer, or `cd` into a directory — whichever the clicked text resolves
//! to (builds on `openurl` and reuses `/view` and `cd`); in chat panes, open a
//! markdown link's URL (`chatview::link_at`) or, when that misses, resolve
//! the clicked token the same way a terminal pane does — so a path an agent
//! cites in a reply is one click away; in the file-viewer pane, open a link
//! on its rendered markdown rung, read off the cell under the click.
use crate::app::CrewApp;
use crate::openurl::{safe_link, url_at};
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
    /// opens in the browser, an existing file opens in the viewer (`e` inside
    /// it reaches `$EDITOR` from there), a directory becomes the new cwd; in
    /// a chat pane, a markdown link opens its URL, and a path the agent wrote
    /// as plain text resolves the same way. Returns `true` when it acted on
    /// something (a miss falls through to the caller's normal click handling
    /// — selection/focus).
    pub(crate) fn cmd_click_at_cursor(&mut self) -> bool {
        // An OSC 8 hyperlink first: its target is what the program said the
        // text points at, and the text is usually prose that no URL scan
        // would find. The status line always names the URL that is actually
        // opening — link text can say one thing and point at another.
        if let Some(uri) = self.cursor_link() {
            return match safe_link(&uri) {
                Some(uri) => {
                    let _ = open::that_detached(uri);
                    self.set_status(format!("opening {uri}"));
                    true
                }
                None => {
                    self.set_status(format!("refused link scheme: {uri}"));
                    true
                }
            };
        }
        if let Some((line, col)) = self.cursor_cell() {
            if let Some(url) = url_at(&line, col) {
                let _ = open::that_detached(&url);
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

    /// Chat/viewer-pane counterpart of the terminal miss above: dispatches on
    /// pane content at the cursor. `None` (nothing under the cursor, or a
    /// pane kind neither arm handles) is a miss, same as the terminal path.
    fn chat_link_click_at_cursor(&mut self) -> bool {
        let Some(i) = self.pane_at_cursor() else {
            return false;
        };
        let Some((row, col)) = self.cursor_rowcol(i) else {
            return false;
        };
        let (row, col) = (row as u16, col as u16);
        match &self.panes[i].content {
            PaneContent::Chat(_) => self.resolve_chat_click(i, row, col),
            PaneContent::View(_) => self.resolve_view_click(i, row, col),
            _ => false,
        }
    }

    /// Chat pane click resolution, in order: a markdown link's URL
    /// (`chatview::link_at`); else the clicked row's plain text run through
    /// `token_at`/`open_path_token` — the same resolution a terminal pane's
    /// Cmd+click gets, so a path an agent wrote in a reply is one click away
    /// without leaving the transcript; else, if neither resolved, the fenced
    /// code block under the click (if any) copies to the clipboard. Row text
    /// is reconstructed lazily, at click time, for this ONE row only — never
    /// pre-scanned during layout.
    fn resolve_chat_click(&mut self, i: usize, row: u16, col: u16) -> bool {
        let grid = self.panes[i].grid;
        let PaneContent::Chat(chat) = &self.panes[i].content else {
            return false;
        };
        if let Some(url) = crate::chatview::link_at(chat, grid.cols, grid.rows, row, col) {
            let _ = open::that_detached(&url);
            self.set_status(format!("opening {url}"));
            return true;
        }
        let token = crate::chatview::row_text_at(chat, grid.cols, grid.rows, row)
            .and_then(|line| token_at(&line, col as usize));
        if let Some(tok) = token {
            if self.open_path_token(&tok) {
                return true;
            }
        }
        let PaneContent::Chat(chat) = &self.panes[i].content else {
            return false;
        };
        let Some(code) = crate::chatview::code_block_at(chat, grid.cols, grid.rows, row) else {
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

    /// File-viewer pane click resolution: a link on its rendered markdown
    /// rung, read off the cell under the click.
    fn resolve_view_click(&mut self, i: usize, row: u16, col: u16) -> bool {
        let pane = &self.panes[i];
        let PaneContent::View(v) = &pane.content else {
            return false;
        };
        let url = {
            let cache = v.lines_for(pane.grid.cols);
            view_link_at(&cache.lines, v.scroll, row, col)
        };
        let Some(url) = url else {
            return false;
        };
        let _ = open::that_detached(&url);
        self.set_status(format!("opening {url}"));
        true
    }

    /// If `tok` resolves (against the cwd) to a file, show it in the viewer;
    /// to a directory, cd.
    pub(crate) fn open_hint_path(&mut self, tok: &str) -> bool {
        self.open_path_token(tok)
    }

    fn open_path_token(&mut self, tok: &str) -> bool {
        let base = if self.cwd.as_os_str().is_empty() {
            std::path::PathBuf::from(".")
        } else {
            self.cwd.clone()
        };
        // `src/main.rs:42` is the shape every compiler, linter and agent
        // prints, and it never opened anything: the position was part of the
        // token, so the file was looked up under a name it does not have.
        let (tok, line) = crate::pathhl::strip_position(tok);
        let p = std::path::Path::new(tok);
        let full = if p.is_absolute() {
            p.to_path_buf()
        } else {
            base.join(p)
        };
        if full.is_file() {
            self.open_view(tok);
            if let Some(n) = line {
                self.goto_last_view(n);
            }
            true
        } else if full.is_dir() {
            self.try_change_dir(&format!("cd {tok}"))
        } else {
            false
        }
    }
}

#[cfg(test)]
#[path = "clickopen_tests.rs"]
mod tests;
