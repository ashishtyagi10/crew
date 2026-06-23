//! `/dump`: export the focused terminal's full scrollback to a timestamped text
//! file in Crew's working directory. Walks the pane from the top of its history
//! down to the live bottom, capturing one line per row, then restores the
//! viewport — useful for archiving an AI agent's output or a long build log.
use std::path::{Path, PathBuf};

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crew_term::{PtyTerm, RenderCell, TermModel};

/// Safety bound on captured lines so a pathological scrollback can't hang Crew.
const MAX_LINES: usize = 50_000;

/// Resolve the dump target: an empty `arg` yields `crew-dump-<stamp>.txt` in
/// `base`; an absolute `arg` is used as-is; a relative `arg` joins `base`.
pub(crate) fn dump_path(arg: &str, base: &Path, stamp: &str) -> PathBuf {
    let arg = arg.trim();
    if arg.is_empty() {
        return base.join(format!("crew-dump-{stamp}.txt"));
    }
    crate::pathexpand::expand_path(base, arg)
}

/// Reconstruct one grid row as a string, trimming trailing blanks.
pub(crate) fn grid_row(cells: &[RenderCell], row: u16, cols: u16) -> String {
    let mut line = vec![' '; cols as usize];
    for c in cells.iter().filter(|c| c.row == row) {
        if (c.col as usize) < line.len() {
            line[c.col as usize] = c.c;
        }
    }
    let s: String = line.into_iter().collect();
    s.trim_end().to_string()
}

/// Walk `pty` from the top of its scrollback to the live bottom, returning the
/// full text. Leaves the viewport where it found it.
pub(crate) fn capture_scrollback(pty: &mut PtyTerm, cols: u16, rows: u16) -> String {
    let start = pty.display_offset();
    // Page up until the offset stops growing — i.e. we hit the oldest line.
    loop {
        let before = pty.display_offset();
        pty.scroll(rows as i32);
        if pty.display_offset() == before {
            break;
        }
    }
    // The top screen contributes every visible row; then each one-line scroll
    // down reveals exactly one new line at the bottom.
    let mut lines: Vec<String> = (0..rows)
        .map(|r| grid_row(&pty.cells(false), r, cols))
        .collect();
    while pty.display_offset() > 0 && lines.len() < MAX_LINES {
        pty.scroll(-1);
        lines.push(grid_row(&pty.cells(false), rows - 1, cols));
    }
    pty.scroll_to_bottom();
    if start > 0 {
        pty.scroll(start as i32);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.push(String::new()); // trailing newline
    lines.join("\n")
}

impl CrewApp {
    /// Save the focused terminal pane's scrollback to a file: `arg` (relative to
    /// the cwd) when given, else a timestamped `crew-dump-*.txt` in the cwd.
    pub(crate) fn dump_focused_pane(&mut self, arg: &str) {
        let focused = self.focused;
        let Some(pane) = self.panes.get_mut(focused) else {
            self.set_status("dump: focus a terminal pane");
            return;
        };
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        let PaneContent::Terminal(t) = &mut pane.content else {
            self.set_status("dump: focus a terminal pane");
            return;
        };
        let text = capture_scrollback(&mut t.pty, cols, rows);
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let dir = if self.cwd.as_os_str().is_empty() {
            std::env::current_dir().unwrap_or_default()
        } else {
            self.cwd.clone()
        };
        let path = dump_path(arg, &dir, &stamp);
        match std::fs::write(&path, &text) {
            Ok(()) => {
                let lines = text.lines().count();
                self.set_status(format!(
                    "dumped {lines} lines ({}) → {}",
                    fmt_bytes(text.len()),
                    path.display()
                ));
            }
            Err(e) => self.set_status(format!("dump failed: {e}")),
        }
    }
}

/// Format a byte count compactly: `512 B`, `12 KB`, `3.4 MB`.
fn fmt_bytes(n: usize) -> String {
    let b = n as f64;
    if b < 1024.0 {
        format!("{n} B")
    } else if b < 1024.0 * 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::{dump_path, fmt_bytes, grid_row};
    use crew_term::RenderCell;

    #[test]
    fn fmt_bytes_units() {
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(2048), "2 KB");
        assert_eq!(fmt_bytes(3_500_000), "3.3 MB");
    }
    use std::path::Path;

    #[test]
    fn dump_path_default_and_explicit() {
        let base = Path::new("/tmp/crewbase");
        // empty arg → timestamped name in the base dir.
        assert_eq!(
            dump_path("  ", base, "20260101-101010"),
            base.join("crew-dump-20260101-101010.txt")
        );
        // a relative arg joins the base; an absolute arg is kept as-is.
        assert_eq!(dump_path("log.txt", base, "s"), base.join("log.txt"));
        assert_eq!(
            dump_path("/var/out.txt", base, "s"),
            Path::new("/var/out.txt")
        );
    }

    fn cell(col: u16, row: u16, c: char) -> RenderCell {
        RenderCell {
            col,
            row,
            c,
            fg: (0, 0, 0),
            bg: (0, 0, 0),
            bold: false,
            italic: false,
        }
    }

    #[test]
    fn grid_row_reconstructs_and_trims() {
        // "hi" on row 1, with a gap and trailing spaces.
        let cells = [cell(0, 1, 'h'), cell(1, 1, 'i'), cell(5, 1, 'x')];
        assert_eq!(grid_row(&cells, 1, 10), "hi   x");
        // an empty row trims to nothing.
        assert_eq!(grid_row(&cells, 0, 10), "");
    }

    #[test]
    fn grid_row_respects_column_bound() {
        // a cell past `cols` is ignored rather than panicking.
        let cells = [cell(0, 0, 'a'), cell(99, 0, 'z')];
        assert_eq!(grid_row(&cells, 0, 3), "a");
    }
}
