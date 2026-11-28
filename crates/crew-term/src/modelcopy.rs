//! Recording OSC 133 marks with the cursor, and cutting a command's output
//! out of the buffer by those lines (see `markpos` for the rule).
use alacritty_terminal::term::TermMode;

use super::TermCore;
use crate::markpos::{CommandOutput, OutputErr, Placed};
use crate::osc::Cut;

/// Longest command text kept with its mark.
const COMMAND_CHARS: usize = 200;
/// Rows of a command line read at most (a pasted heredoc is not a title).
const COMMAND_ROWS: u64 = 8;

impl TermCore {
    /// A mark ended just now: note where the cursor is. The parser has been
    /// fed up to the mark's last byte and not past it.
    pub(super) fn mark_here(&mut self, cut: Cut) {
        let cursor = &self.term.grid().cursor;
        let point = cursor.point;
        // A row exactly full leaves the cursor on its last column, waiting
        // to wrap: it is past the text, not on it.
        let col = point.column.0 + usize::from(cursor.input_needs_wrap);
        let (line, col) = (self.screen_line() + point.line.0.max(0) as u64, col as u16);
        let mut p = Placed {
            cut,
            line,
            col,
            alt: self.term.mode().contains(TermMode::ALT_SCREEN),
            exact: self.parser.sync_bytes_count() == 0,
            epoch: self.marks.epoch(),
            from: line.saturating_sub(1),
            text: String::new(),
        };
        match cut {
            Cut::Begin => self.marks.begin = Some((line, col)),
            Cut::Prompt => self.marks.begin = None,
            Cut::Output => {
                if let Some((bl, bc)) = self.marks.begin.take() {
                    p.from = bl;
                    p.text = self.typed(bl, bc, line, col);
                }
            }
            Cut::Done => {}
        }
        self.marks.push(p);
    }

    /// The command line between `B` and `C`, one line.
    fn typed(&self, bl: u64, bc: u16, cl: u64, cc: u16) -> String {
        let stop = (cl + u64::from(cc > 0)).min(bl + COMMAND_ROWS);
        let mut out = String::new();
        for abs in bl..stop {
            let from = if abs == bl { usize::from(bc) } else { 0 };
            let Some((t, wrapped)) = self.line_text(abs, from, usize::MAX) else {
                break;
            };
            out.push_str(&t);
            if !wrapped {
                out = out.trim_end().to_string();
                out.push(' ');
            }
        }
        out.trim().chars().take(COMMAND_CHARS).collect()
    }

    /// The text the last finished command (or, scrolled back, the one on
    /// view) printed. Refuses rather than guesses when its lines cannot be
    /// trusted: alternate screen, a width change since, a held-back cursor.
    pub(crate) fn command_output(&self) -> Result<CommandOutput, OutputErr> {
        if self.term.mode().contains(TermMode::ALT_SCREEN) {
            return Err(OutputErr::AltScreen);
        }
        let off = self.display_offset() as u64;
        let top = (off > 0).then(|| self.screen_line() - off);
        let pair = self.marks.pick(top).ok_or(OutputErr::NoMarks)?;
        let (out, end) = (pair.out, pair.end);
        if out.alt || end.alt {
            return Err(OutputErr::AltScreen);
        }
        if out.epoch != self.marks.epoch() || end.epoch != self.marks.epoch() {
            return Err(OutputErr::Resized);
        }
        if !out.exact || !end.exact {
            return Err(OutputErr::Unsure);
        }
        let command = out.text.clone();
        let (start, stop) = (pair.start(), pair.stop());
        // After `clear` the closing mark can sit above the opening one.
        if stop <= start {
            return Err(OutputErr::Nothing(command));
        }
        // The oldest line still held is the number of lines thrown away.
        if stop <= self.evicted {
            return Err(OutputErr::Gone(command));
        }
        let first = start.max(self.evicted);
        let dropped = first - start;
        let tail = usize::from(end.col) * usize::from(end.col > 0);
        let text = self.lines_text(first, stop, tail);
        match text.is_empty() {
            true => Err(OutputErr::Nothing(command)),
            false => Ok(CommandOutput {
                command,
                text,
                dropped,
            }),
        }
    }

    /// Absolute lines `[from, to)` as text, the last cut at column `tail`
    /// (0: whole): a soft-wrapped row continues on
    /// the next with its trailing spaces kept (only the end of a LOGICAL
    /// line is trimmed), and blank lines at the end are dropped. Rows are
    /// read in place, so the cost is the range, not the scrollback.
    fn lines_text(&self, from: u64, to: u64, tail: usize) -> String {
        let (mut lines, mut cur) = (Vec::<String>::new(), String::new());
        for abs in from..to {
            // A prompt drawn on the same row (no newline after the output)
            // is not output: the last row stops where `D` found the cursor.
            let end = if abs + 1 == to && tail > 0 {
                tail
            } else {
                usize::MAX
            };
            let Some((t, wrapped)) = self.line_text(abs, 0, end) else {
                break;
            };
            match wrapped {
                true => cur.push_str(&t),
                false => {
                    cur.push_str(t.trim_end());
                    lines.push(std::mem::take(&mut cur));
                }
            }
        }
        if !cur.is_empty() {
            lines.push(cur.trim_end().to_string());
        }
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }
}

#[cfg(test)]
#[path = "modelcopy_tests.rs"]
mod modelcopy_tests;
#[cfg(test)]
#[path = "modelcopyedge_tests.rs"]
mod modelcopyedge_tests;
