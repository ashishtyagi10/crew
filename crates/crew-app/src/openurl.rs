//! `/open`: open a URL or path with the OS default app. With no argument, opens
//! the most recent http(s) URL visible in the focused terminal. Also powers
//! Cmd+click on a URL in a terminal pane (see `open_url_at_cursor`).
use crate::app::CrewApp;
use crate::dump::grid_row;
use crate::pane::PaneContent;
use crew_term::TermModel;

/// Characters trimmed from a URL's tail (trailing punctuation in prose).
const TRAILERS: &str = ".,);]}>\"'";

/// Extract http/https URLs from `text`, in order of appearance.
pub(crate) fn find_urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("http") {
        let cand = &rest[i..];
        if cand.starts_with("http://") || cand.starts_with("https://") {
            let end = cand.find(char::is_whitespace).unwrap_or(cand.len());
            let url = cand[..end].trim_end_matches(|c| TRAILERS.contains(c));
            if url.len() > "https://".len() {
                out.push(url.to_string());
            }
            rest = &cand[end..];
        } else {
            rest = &cand[4..]; // step past this non-URL "http"
        }
    }
    out
}

/// Character spans `[start, end)` of the http(s) URLs in `chars` (one row of a
/// terminal grid). Trailing prose punctuation is excluded from each span.
pub(crate) fn url_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let tail: String = chars[i..].iter().take(8).collect();
        if tail.starts_with("http://") || tail.starts_with("https://") {
            let mut j = i;
            while j < chars.len() && !chars[j].is_whitespace() {
                j += 1;
            }
            let mut end = j;
            while end > i && TRAILERS.contains(chars[end - 1]) {
                end -= 1;
            }
            if end - i > "https://".len() {
                spans.push((i, end));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    spans
}

/// Returns the http(s) URL spanning character column `col` in `line`, if `col`
/// falls inside one. Used to resolve a Cmd+click to a link.
pub(crate) fn url_at(line: &str, col: usize) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    url_spans(&chars)
        .into_iter()
        .find(|&(a, b)| (a..b).contains(&col))
        .map(|(a, b)| chars[a..b].iter().collect())
}

impl CrewApp {
    /// Open `arg` (URL or path) with the OS default; an empty `arg` opens the
    /// most recent URL visible in the focused terminal.
    pub(crate) fn open_target(&mut self, arg: &str) {
        let arg = arg.trim();
        let target = if arg.is_empty() {
            match self.last_visible_url() {
                Some(url) => url,
                None => {
                    self.set_status("no URL on screen — try /open <url|path>");
                    return;
                }
            }
        } else {
            self.resolve_open(arg)
        };
        match open::that(&target) {
            Ok(()) => self.set_status(format!("opening {target}")),
            Err(e) => self.set_status(format!("open failed: {e}")),
        }
    }

    /// Resolve `arg` to something openable: URLs pass through; a path is expanded
    /// (`~`, `$VAR`) and resolved against Crew's working directory.
    fn resolve_open(&self, arg: &str) -> String {
        if arg.contains("://") {
            return arg.to_string();
        }
        crate::pathexpand::expand_path(&self.cwd, arg)
            .to_string_lossy()
            .into_owned()
    }

    /// The last http(s) URL visible in the focused terminal pane, if any.
    fn last_visible_url(&self) -> Option<String> {
        let pane = self.panes.get(self.focused)?;
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        let PaneContent::Terminal(t) = &pane.content else {
            return None;
        };
        let cells = t.pty.cells(false);
        let text = (0..rows)
            .map(|r| grid_row(&cells, r, cols))
            .collect::<Vec<_>>()
            .join("\n");
        find_urls(&text).pop()
    }

    /// The row text and character column under the cursor in a terminal pane
    /// (content rows only; the title bar is excluded). Drives Cmd+click.
    pub(crate) fn cursor_cell(&self) -> Option<(String, usize)> {
        let i = self.pane_at_cursor()?;
        let (cw, ch, _sw, _sh, _scale) = self.frame_geometry()?;
        let rect = self.grid_rects().get(i).copied()?;
        let col = ((self.cursor.0 - rect.x) / cw).floor() as i32;
        // Content sits one row below the pane's title bar.
        let row = ((self.cursor.1 - rect.y) / ch).floor() as i32 - 1;
        if col < 0 || row < 0 {
            return None;
        }
        let pane = &self.panes[i];
        let PaneContent::Terminal(t) = &pane.content else {
            return None;
        };
        let line = grid_row(&t.pty.cells(false), row as u16, pane.grid.cols);
        Some((line, col as usize))
    }
}

#[cfg(test)]
#[path = "openurl_tests.rs"]
mod tests;
