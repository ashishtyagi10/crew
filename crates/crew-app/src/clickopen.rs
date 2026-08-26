//! Cmd/Ctrl+click resolution: in terminal panes, open a URL, show a file in
//! the viewer, or `cd` into a directory — whichever the clicked text resolves
//! to (builds on `openurl` and reuses `/view` and `cd`); in chat panes, open a
//! markdown link's URL (`chatview::link_at`) or, when that misses, resolve
//! the clicked token the same way a terminal pane does — so a path an agent
//! cites in a reply is one click away; in the file-viewer pane, open a link
//! on its rendered markdown rung, read off the cell under the click.
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
    /// opens in the browser, an existing file opens in the viewer (`e` inside
    /// it reaches `$EDITOR` from there), a directory becomes the new cwd; in
    /// a chat pane, a markdown link opens its URL, and a path the agent wrote
    /// as plain text resolves the same way. Returns `true` when it acted on
    /// something (a miss falls through to the caller's normal click handling
    /// — selection/focus).
    pub(crate) fn cmd_click_at_cursor(&mut self) -> bool {
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
            self.open_view(tok);
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
    use crate::app::CrewApp;
    use crate::chatbody::{plain, CardCell, CardLine};
    use crate::pane::PaneContent;

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

    #[test]
    fn a_path_an_agent_wrote_opens_the_viewer() {
        // The file the agent just changed should be one click away, without
        // leaving the transcript.
        let dir = std::env::temp_dir();
        let f = dir.join("agent-cited.rs");
        std::fs::write(&f, "fn main() {}\n").unwrap();

        let mut chat = crate::chat::tests::pane();
        chat.push_capped(crate::chatlayout::Message {
            sender: "agent smith".into(),
            text: "see agent-cited.rs for the fix".into(),
            ts: "1".into(),
            meta: String::new(),
            usage: None,
            expanded: false,
        });
        let (cols, rows) = (80u16, 20u16);
        // Locate where the path actually rendered rather than hardcoding
        // layout — mirrors `chatview_tests`' own click-target lookups.
        let mut rendered: std::collections::BTreeMap<u16, String> = Default::default();
        for c in crate::chatview::cells(&chat, cols, rows) {
            rendered.entry(c.row).or_default().push(c.c);
        }
        let (&row, line) = rendered
            .iter()
            .find(|(_, text)| text.contains("agent-cited.rs"))
            .expect("the path text rendered somewhere");
        let col = line.find("agent-cited.rs").unwrap() as u16;

        let mut app = CrewApp {
            cwd: dir.clone(),
            ..Default::default()
        };
        app.panes.push(crate::pane::Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Chat(chat),
            grid: crew_term::GridSize { cols, rows },
            rect: crate::layout::Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        });
        let before = app.panes.len();

        assert!(
            app.resolve_chat_click(0, row, col),
            "clicking the cited path must act"
        );
        assert_eq!(app.panes.len(), before + 1, "a new pane opened");
        assert!(
            app.panes
                .iter()
                .any(|p| matches!(&p.content, PaneContent::View(v) if v.path == f)),
            "the viewer opened on the exact file the agent cited"
        );
    }
}
