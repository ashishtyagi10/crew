//! Which span an OSC 133 `D` (command finished, exit status) belongs to.
//!
//! Spans open only when the once-a-second poll sees a foreground command, so
//! a command faster than a poll (`ls`, `cd`, `false`) has no span, yet its
//! `D` still arrives. Stamping it on "the last span" blamed the wrong
//! command: a fast success wiped the failure before it, a fast failure was
//! pinned on an earlier success, and `/blocks` and `??` read the lie.
use crate::cmdspan::Spans;

impl Spans {
    /// The shell said a command's output began (`C`) at monotonic `now`.
    /// Remembered so [`Self::finished`] can tell a `D` from a command that
    /// started AFTER the last span closed (and never got a span) from one
    /// that belongs to it.
    pub(crate) fn output_started(&mut self, now: u64) {
        self.1 = Some(now);
    }

    /// The shell reported that a command finished with `code`, at buffer
    /// line `at` (OSC 133 `D`).
    ///
    /// It closes the span as well as marking it: the shell knows the command
    /// is over a full poll before the foreground-process watch will notice,
    /// and a boundary the shell states beats one crew inferred. Applied to
    /// the LAST span only when that is the command the `D` is for:
    /// * never when it already has an exit (a `D` is stated once; a second is
    ///   a different, span-less command);
    /// * not when it is closed and the latest `C` came after it closed — that
    ///   command started after this span ended, so it had no span of its own;
    /// * with no `C` ever seen (a shell that sends only `D`) the first rule
    ///   is all there is to go on.
    ///
    /// A `D` landing just after the poll closed the span, with its `C` before
    /// the close, is the same command and still lands.
    pub(crate) fn finished(&mut self, code: Option<i32>, at: usize, now: u64) {
        let c_at = self.1;
        let Some(last) = self.0.last_mut().filter(|s| s.exit.is_none()) else {
            return;
        };
        if let (Some(done), Some(c)) = (last.done_ms, c_at) {
            if c > done {
                return;
            }
        }
        if last.to.is_none() {
            last.to = Some(at.max(last.from));
            last.done_ms = Some(now);
        }
        last.exit = code;
    }
}

#[cfg(test)]
#[path = "cmdexit_tests.rs"]
mod tests;
