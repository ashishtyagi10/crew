//! Where each command's output starts and ends in a pane's buffer.
//!
//! Crew watches each pane's foreground process once a second; the two edges
//! it sees (idle to running, running to idle) bound a command's output, and
//! the buffer's line count at each is the span. Second-granularity: a command
//! faster than a poll leaves no span. It buys `/out` without asking anyone to
//! change their shell. How a command *ended* polling cannot see: an OSC 133
//! shell says so (`D;1`), [`Spans::finished`] records it, and the border marks
//! that block's first row. A silent shell keeps what it had.

/// One command's output, as buffer line indices.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Span {
    /// What was running.
    pub name: String,
    /// `lines_seen` when it started and stopped (evicted lines count, so a
    /// mark outlives a full scrollback). `to` is `None` while it is running.
    pub from: usize,
    pub to: Option<usize>,
    /// The exit status the shell reported for it (OSC 133 `D`). `None` for
    /// every shell with no integration configured — which is not the same as
    /// "it succeeded", and is why nothing is drawn for it.
    pub exit: Option<i32>,
    /// The whole command line (`cargo test -p crew-app`) when the process
    /// table gave one; `name` alone says only `cargo`.
    pub line: Option<String>,
    /// The monotonic clock when it started and when it stopped. Buffer lines
    /// say where the output IS; these say how long it took, which is the
    /// other half of "what did I run in here" (see [`crate::blocks`]).
    pub at_ms: u64,
    pub done_ms: Option<u64>,
}

/// How many spans a pane remembers. Enough to reach back through a working
/// session; small enough to stay a rounding error next to the scrollback.
const CAP: usize = 32;

/// The spans a pane has seen, oldest first.
#[derive(Default)]
pub(crate) struct Spans(
    pub(crate) Vec<Span>,
    /// When the shell last said a command's output began (OSC 133 `C`), on
    /// the monotonic clock; see [`crate::cmdexit`].
    pub(crate) Option<u64>,
);

impl Spans {
    /// A command started at buffer line `at`, on the monotonic clock `now`.
    pub(crate) fn started(&mut self, name: String, at: usize, now: u64) {
        // An unclosed span means the previous command's end was missed; close
        // it here rather than leaving a span that runs to the end of time.
        self.close_at(at, now);
        self.0.push(Span {
            name,
            from: at,
            to: None,
            exit: None,
            line: None,
            at_ms: now,
            done_ms: None,
        });
        while self.0.len() > CAP {
            self.0.remove(0);
        }
    }

    /// Record the command line of the span just opened; no line leaves it.
    pub(crate) fn set_line(&mut self, line: Option<String>) {
        if let (Some(s), Some(line)) = (self.0.last_mut(), line) {
            s.line = Some(line);
        }
    }

    /// The running command stopped at buffer line `at`, on the monotonic
    /// clock `now`.
    pub(crate) fn close_at(&mut self, at: usize, now: u64) {
        if let Some(open) = self.0.last_mut().filter(|s| s.to.is_none()) {
            open.to = Some(at.max(open.from));
            open.done_ms = Some(now);
        }
    }

    /// How long a span ran, or has been running.
    pub(crate) fn elapsed_ms(s: &Span, now: u64) -> u64 {
        s.done_ms.unwrap_or(now).saturating_sub(s.at_ms)
    }

    /// The exit status of the most recent command, when the shell reported
    /// one. `None` (a silent shell) is not a success.
    pub(crate) fn last_exit(&self) -> Option<i32> {
        self.0.last()?.exit
    }

    /// Every span, newest first — what [`crate::blocks`] lists.
    pub(crate) fn recent(&self) -> impl Iterator<Item = &Span> {
        self.0.iter().rev()
    }

    /// The visible rows that begin a command the shell reported FAILED, each
    /// with its span. Window arithmetic as [`Self::start_rows`]; the card
    /// draws these rows and a click on one reads the span, so both agree.
    pub(crate) fn failed_in_view(
        &self,
        now: usize,
        visible: usize,
        scroll: usize,
    ) -> Vec<(u16, &Span)> {
        let first = now.saturating_sub(visible).saturating_sub(scroll);
        let row =
            |s: &Span| u16::try_from(s.from.checked_sub(first).filter(|r| *r < visible)?).ok();
        let failed = |s: &&Span| s.exit.is_some_and(|c| c != 0);
        let spans = self.0.iter().filter(failed);
        spans.filter_map(|s| Some((row(s)?, s))).collect()
    }

    /// Whether this span has any output in a buffer of `now` lines.
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

    /// What a pane has run lately, newest first.
    pub(crate) fn summary(&self, limit: usize) -> Vec<String> {
        self.0
            .iter()
            .rev()
            .take(limit)
            .enumerate()
            .map(|(i, s)| format!("{i}:{}", s.name))
            .collect()
    }

    /// The `[from, to)` line range of `span` in a buffer of `now` lines,
    /// clamped to it: scrollback is evicted under us.
    pub(crate) fn range(span: &Span, now: usize) -> (usize, usize) {
        let to = span.to.unwrap_or(now).min(now);
        (span.from.min(to), to)
    }

    /// The command whose output covers buffer line `line` in a buffer of
    /// `now` lines — what [`crate::cmdhead`] names on the top border while a
    /// pane is scrolled back.
    ///
    /// Newest-first, so a pair overlapping by the poll's granularity answers
    /// with the later one.
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
