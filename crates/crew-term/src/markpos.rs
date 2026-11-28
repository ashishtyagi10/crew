//! Where each OSC 133 mark landed, so a command's output is a range of
//! buffer lines rather than a guess from the process poll.
//!
//! The scanner says which byte ended a mark (`osc::Cut`), `feed_split` feeds
//! the parser up to it, and `modelcopy` reads the cursor there: the same
//! trick pictures use (`modelimage`), and exact for the same reason, since
//! nothing past the mark has been parsed yet. The numbers are ABSOLUTE lines
//! (`TermCore::screen_line` plus the cursor row), so they survive scrolling
//! and a full scrollback's eviction.
//!
//! What a command's output is: every shell integration (zsh, bash PS0, fish,
//! kitty's, iTerm2's, VS Code's) prints `C` from its preexec hook after the
//! user's Enter, which has already moved the cursor to column 0 of the line
//! after the command line, and `D` from precmd after the program exited.
//! So the output is the lines from `C` up to `D`'s line, exclusive when `D`
//! is at column 0; a cursor left mid-line (`C` before the newline; output
//! with no final newline) is read as "that line belongs too" on the `D` side
//! and "starts on the next line" on the `C` side. A prompt (`A`) with no `D`
//! before it closes the command too, with no status: the shell redrew its
//! prompt, so the output is over, and it is still copyable.
use std::collections::VecDeque;

use crate::osc::Cut;

/// Marks kept: a few per command, so this is dozens of commands, and what a
/// scroll back to an old one can still reach.
const KEEP: usize = 256;

/// One mark and the cursor where it arrived.
pub(crate) struct Placed {
    pub cut: Cut,
    pub line: u64,
    pub col: u16,
    /// Arrived while the alternate screen was up: its line means nothing on
    /// the primary buffer's numbering.
    pub alt: bool,
    /// False when a synchronized update was holding bytes back, so the
    /// cursor read there is older than the mark.
    pub exact: bool,
    /// [`Marks::epoch`] when it arrived.
    pub epoch: u32,
    /// `C` only: the line the command was typed on.
    pub from: u64,
    /// `C` only: the command as typed, read at the mark (`B` to `C`) because
    /// by the time anyone asks it can be scrolled away or cleared.
    pub text: String,
}

#[derive(Default)]
pub(crate) struct Marks {
    list: VecDeque<Placed>,
    /// Bumped when the width changes: the text was rewrapped, so every line
    /// number recorded before is stale (80→60→80 does not undo it).
    epoch: u32,
    /// Where the command line began (`B`), until its `C` or the next `A`.
    pub begin: Option<(u64, u16)>,
}

/// A finished command: its `C` mark and the mark that closed it.
pub(crate) struct Pair<'a> {
    pub out: &'a Placed,
    pub end: &'a Placed,
}

impl Pair<'_> {
    /// First output line: the next one when `C` came mid-line.
    pub fn start(&self) -> u64 {
        self.out.line + u64::from(self.out.col > 0)
    }

    /// One past the last output line.
    pub fn stop(&self) -> u64 {
        self.end.line + u64::from(self.end.col > 0)
    }
}

impl Marks {
    pub fn push(&mut self, p: Placed) {
        self.list.push_back(p);
        while self.list.len() > KEEP {
            self.list.pop_front();
        }
    }

    /// Every mark as `(cut, line, column)`, for tests that compare runs.
    #[cfg(test)]
    pub fn dump(&self) -> Vec<(Cut, u64, u16)> {
        self.list.iter().map(|m| (m.cut, m.line, m.col)).collect()
    }

    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    pub fn reflowed(&mut self) {
        self.epoch += 1;
    }

    /// Every command with a `C` that something closed, oldest first. A new
    /// `C` before any close abandons the one before it (the shell lost it).
    fn pairs(&self) -> Vec<Pair<'_>> {
        let (mut out, mut open) = (Vec::new(), None);
        for m in &self.list {
            match (m.cut, open) {
                (Cut::Output, _) => open = Some(m),
                (Cut::Done | Cut::Prompt, Some(c)) => {
                    out.push(Pair { out: c, end: m });
                    open = None;
                }
                _ => {}
            }
        }
        out
    }

    /// The command to copy: the newest finished one, or while scrolled back
    /// (`top` is the absolute line on the view's top row) the newest whose
    /// command line is at or above that row — the one whose text or output
    /// the view is showing, after a wheel scroll and after Cmd+Opt+↑/↓ alike.
    pub fn pick(&self, top: Option<u64>) -> Option<Pair<'_>> {
        let seen = |p: &Pair| top.is_none_or(|t| p.out.from <= t);
        self.pairs().into_iter().rev().find(seen)
    }
}

/// A command's output, ready for the clipboard.
#[derive(Debug, PartialEq, Eq)]
pub struct CommandOutput {
    /// As typed, one line.
    pub command: String,
    pub text: String,
    /// Lines at the head the scrollback had already thrown away.
    pub dropped: u64,
}

/// Why there is nothing to copy.
#[derive(Debug, PartialEq, Eq)]
pub enum OutputErr {
    /// The shell never sent a finished command (no integration).
    NoMarks,
    /// A full-screen program is up, or ran: the buffer is not the log.
    AltScreen,
    /// The width changed since: the lines were rewrapped.
    Resized,
    /// A synchronized update held the cursor back at a mark.
    Unsure,
    /// It printed nothing.
    Nothing(String),
    /// All of it was thrown away with the scrollback.
    Gone(String),
}
