//! Where each command's output starts and ends in a pane's buffer.
//!
//! Terminals that speak OSC 133 learn this from the shell. Crew does not need
//! the shell to tell it: it already watches the foreground process of every
//! pane once a second, so the two transitions it sees — idle to running, and
//! running back to idle — are exactly the two edges of a command's output.
//! The buffer's line count at each edge is the span.
//!
//! Second-granularity, and honest about it: a command that starts and ends
//! between two polls leaves no span, and one whose output is still arriving
//! when the prompt returns can carry a line or two of the next thing. What it
//! buys is `/out` — the last command's output on its own, in a pane you can
//! read — without asking anyone to change their shell configuration.

/// One command's output, as buffer line indices.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Span {
    /// What was running.
    pub name: String,
    /// Buffer lines when it started, and when it stopped. `to` is `None`
    /// while it is still running.
    pub from: usize,
    pub to: Option<usize>,
}

/// How many spans a pane remembers. Enough to reach back through a working
/// session; small enough to stay a rounding error next to the scrollback.
const CAP: usize = 32;

/// The spans a pane has seen, oldest first.
#[derive(Default)]
pub(crate) struct Spans(Vec<Span>);

impl Spans {
    /// A command started at buffer line `at`.
    pub(crate) fn started(&mut self, name: String, at: usize) {
        // An unclosed span means the previous command's end was missed; close
        // it here rather than leaving a span that runs to the end of time.
        self.close(at);
        self.0.push(Span {
            name,
            from: at,
            to: None,
        });
        while self.0.len() > CAP {
            self.0.remove(0);
        }
    }

    /// The running command stopped at buffer line `at`.
    pub(crate) fn close(&mut self, at: usize) {
        if let Some(open) = self.0.last_mut().filter(|s| s.to.is_none()) {
            open.to = Some(at.max(open.from));
        }
    }

    /// Whether this span has any output in a buffer of `now` lines. A
    /// command that has printed nothing — still running, or finished without
    /// a word — is not something to show.
    fn has_output(s: &Span, now: usize) -> bool {
        s.to.unwrap_or(now) > s.from
    }

    /// The `n`th span with output, counting back from the most recent (`0` is
    /// the latest). Spans that printed nothing are skipped rather than
    /// counted, so `/out 1` means "the command before the one `/out` shows"
    /// rather than "the one before whatever happens to be last".
    pub(crate) fn nth_back(&self, n: usize, now: usize) -> Option<&Span> {
        self.0
            .iter()
            .rev()
            .filter(|s| Self::has_output(s, now))
            .nth(n)
    }

    /// What a pane has run lately, newest first — the answer `/out` gives
    /// when asked for a command that is not there.
    pub(crate) fn summary(&self, limit: usize) -> Vec<String> {
        self.0
            .iter()
            .rev()
            .take(limit)
            .enumerate()
            .map(|(i, s)| format!("{i}:{}", s.name))
            .collect()
    }

    /// The `[from, to)` line range of `span` in a buffer that currently holds
    /// `now` lines, clamped to it — the buffer wraps its scrollback away
    /// under us, and a range past the end would slice nothing.
    pub(crate) fn range(span: &Span, now: usize) -> (usize, usize) {
        let to = span.to.unwrap_or(now).min(now);
        (span.from.min(to), to)
    }

    /// The command whose output covers buffer line `line` in a buffer of
    /// `now` lines — what [`crate::cmdhead`] names on the top border while a
    /// pane is scrolled back.
    ///
    /// Searched newest-first so an overlapping pair (the poll's one-second
    /// granularity can close a span a line or two into the next command's
    /// output) answers with the later one: the top of your window is inside
    /// the thing that printed most recently, not the thing it interrupted.
    /// A line in no span at all — before the first command crew saw, or in
    /// the gap between two — has no honest answer and gets `None`.
    pub(crate) fn at_line(&self, line: usize, now: usize) -> Option<&Span> {
        self.0.iter().rev().find(|s| {
            let (from, to) = Self::range(s, now);
            line >= from && line < to
        })
    }

    /// Which visible rows a command *started* on, given a window showing
    /// `visible` rows ending `scroll` lines back from a buffer of `now`
    /// lines. The pane's card ticks these on its left border: where one thing
    /// you ran ends and the next begins, without a shell integration and
    /// without spending a column of the program's own grid.
    pub(crate) fn start_rows(&self, now: usize, visible: usize, scroll: usize) -> Vec<u16> {
        let first = now.saturating_sub(visible).saturating_sub(scroll);
        self.0
            .iter()
            .filter_map(|s| s.from.checked_sub(first))
            .filter(|row| *row < visible)
            .map(|row| row as u16)
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
#[path = "cmdspan_tests.rs"]
mod tests;
