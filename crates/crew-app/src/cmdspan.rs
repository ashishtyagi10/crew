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
//!
//! ## The one thing polling cannot see
//!
//! How the command *ended*. A process crew never saw start tells it nothing
//! about its exit status, and no amount of polling recovers one. A shell with
//! an OSC 133 integration says so directly (`ESC ] 133 ; D ; 1 ST`), and when
//! it does, [`Spans::finished`] records it and the pane's border marks that
//! block's first row in the alarm colour. A shell that says nothing keeps
//! exactly the blocks it had before — this is an upgrade, not a requirement.

/// One command's output, as buffer line indices.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Span {
    /// What was running.
    pub name: String,
    /// Buffer lines when it started, and when it stopped. `to` is `None`
    /// while it is still running.
    pub from: usize,
    pub to: Option<usize>,
    /// The exit status the shell reported for it (OSC 133 `D`). `None` for
    /// every shell with no integration configured — which is not the same as
    /// "it succeeded", and is why nothing is drawn for it.
    pub exit: Option<i32>,
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
pub(crate) struct Spans(Vec<Span>);

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
            at_ms: now,
            done_ms: None,
        });
        while self.0.len() > CAP {
            self.0.remove(0);
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

    /// How long a span ran, or how long it has been running. `None` for one
    /// that started before the clock was being kept.
    pub(crate) fn elapsed_ms(s: &Span, now: u64) -> u64 {
        s.done_ms.unwrap_or(now).saturating_sub(s.at_ms)
    }

    /// The exit status of the most recent command, when the shell reported
    /// one. `None` for a pane whose shell says nothing — which is not the
    /// same as a success, and is why the caller must not treat it as one.
    pub(crate) fn last_exit(&self) -> Option<i32> {
        self.0.last()?.exit
    }

    /// Every span, newest first — what [`crate::blocks`] lists.
    pub(crate) fn recent(&self) -> impl Iterator<Item = &Span> {
        self.0.iter().rev()
    }

    /// The shell reported that the last command finished with `code`, at
    /// buffer line `at` (OSC 133 `D`).
    ///
    /// It closes the span as well as marking it: the shell knows the command
    /// is over a full poll before the foreground-process watch will notice,
    /// and a boundary the shell states beats one crew inferred. Applied to
    /// the LAST span whether or not it is still open — a `D` arriving just
    /// after the poll closed the span is the same command, and the alternative
    /// is dropping the one fact polling cannot supply.
    pub(crate) fn finished(&mut self, code: Option<i32>, at: usize, now: u64) {
        let Some(last) = self.0.last_mut() else {
            return;
        };
        if last.to.is_none() {
            last.to = Some(at.max(last.from));
            last.done_ms = Some(now);
        }
        last.exit = code;
    }

    /// Which visible rows hold the first line of a command that FAILED. Same
    /// window arithmetic as [`Self::start_rows`], over the spans a shell
    /// reported a non-zero status for.
    pub(crate) fn failed_rows(&self, now: usize, visible: usize, scroll: usize) -> Vec<u16> {
        let first = now.saturating_sub(visible).saturating_sub(scroll);
        self.0
            .iter()
            .filter(|s| s.exit.is_some_and(|c| c != 0))
            .filter_map(|s| s.from.checked_sub(first))
            .filter(|row| *row < visible)
            .map(|row| row as u16)
            .collect()
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
