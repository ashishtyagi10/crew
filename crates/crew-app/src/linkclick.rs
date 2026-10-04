//! What a Cmd+click in a terminal pane is ON, and where it leads.
//!
//! Three things the row under the pointer cannot say by itself. A URL or a
//! path longer than the pane is written across two rows, so the click reads
//! the LOGICAL line — the rows the terminal says soft-wrap onto each other,
//! joined — or it opens half a URL. A file reference is the span the hover
//! underlined (`pathhl`), not the whitespace word around it: Claude Code
//! writes `Update(src/main.rs)`, and the word is not a file. And a relative
//! path is relative to the directory of the pane that printed it, not to
//! crew's own: an agent in `~/code/app` cites `src/main.rs` from there.
use std::path::{Path, PathBuf};

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crew_term::TermModel;

/// The logical line through viewport `row` of `rows` — that row joined with
/// the rows it soft-wraps from and onto (`wraps[r]`: row `r` continues on
/// `r + 1`) — with `col` as an index into it, and the row the line starts on.
pub(crate) fn logical_line(
    rows: &[Vec<char>],
    wraps: &[bool],
    row: usize,
    col: usize,
) -> Option<(Vec<char>, usize, usize)> {
    rows.get(row)?;
    let wrapped = |r: usize| wraps.get(r).copied().unwrap_or(false);
    let mut first = row;
    while first > 0 && wrapped(first - 1) {
        first -= 1;
    }
    let mut last = row;
    while last + 1 < rows.len() && wrapped(last) {
        last += 1;
    }
    let at: usize = rows[first..row].iter().map(Vec::len).sum::<usize>() + col;
    let line: Vec<char> = rows[first..=last].iter().flatten().copied().collect();
    Some((line, at, first))
}

/// The file reference spanning `col` in `line`, as the hover marked it.
pub(crate) fn path_at(line: &[char], col: usize) -> Option<String> {
    crate::pathhl::path_spans(line)
        .into_iter()
        .find(|&(a, b)| (a..b).contains(&col))
        .map(|(a, b)| line[a..b].iter().collect())
}

/// Hand `target` to the system opener — the browser for a URL. Tests record
/// it instead (`OPENED`), so a click can be followed end to end without a
/// browser window opening on the machine running them.
pub(crate) fn open_external(target: &str) {
    #[cfg(test)]
    OPENED.with(|o| o.borrow_mut().push(target.to_string()));
    #[cfg(not(test))]
    let _ = open::that_detached(target);
}

#[cfg(test)]
thread_local! {
    pub(crate) static OPENED: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

impl CrewApp {
    /// The logical line under the cursor in a terminal pane and the cursor's
    /// index into it, plus the pane and the viewport row the line starts on.
    pub(crate) fn cursor_line(&self) -> Option<(usize, Vec<char>, usize, usize)> {
        let i = self.pane_at_cursor()?;
        let (row, col) = self.cursor_rowcol(i)?;
        let pane = &self.panes[i];
        let PaneContent::Terminal(t) = &pane.content else {
            return None;
        };
        let (cols, n) = (usize::from(pane.grid.cols), usize::from(pane.grid.rows));
        let mut rows = vec![vec![' '; cols]; n];
        for c in t.pty.cells(false) {
            if let Some(slot) = rows
                .get_mut(usize::from(c.row))
                .and_then(|r| r.get_mut(usize::from(c.col)))
            {
                *slot = c.c;
            }
        }
        let wraps = TermModel::wrapped_rows(&t.pty);
        let (row, col) = (usize::try_from(row).ok()?, usize::try_from(col).ok()?);
        if col >= cols {
            return None;
        }
        let (line, at, first) = logical_line(&rows, &wraps, row, col)?;
        Some((i, line, at, first))
    }

    /// If `tok` names a file, show it in the viewer (at its `:line`, if it
    /// gave one); a directory becomes the new cwd. Relative to `dir` — the
    /// clicked pane's own directory — first, then to crew's.
    pub(crate) fn open_path_token(&mut self, tok: &str, dir: Option<&Path>) -> bool {
        // `src/main.rs:42` is the shape every compiler, linter and agent
        // prints, and it never opened anything: the position was part of the
        // token, so the file was looked up under a name it does not have.
        let (tok, line) = crate::pathhl::strip_position(tok);
        let own = match self.cwd.as_os_str().is_empty() {
            true => PathBuf::from("."),
            false => self.cwd.clone(),
        };
        let full = dir
            .into_iter()
            .chain([own.as_path()])
            .map(|base| crate::pathexpand::expand_path(base, tok))
            .find(|p| p.exists());
        let Some(full) = full else {
            return false;
        };
        if full.is_file() {
            self.open_view(&full.to_string_lossy());
            if let Some(n) = line {
                self.goto_last_view(n);
            }
            true
        } else {
            self.try_change_dir(&format!("cd {}", full.display()))
        }
    }
}

#[cfg(test)]
#[path = "linkclick_tests.rs"]
mod tests;
