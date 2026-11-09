//! Corrections are remembered: what the user ruled out, and what a long turn
//! found did not work, written into the recall graph so the next task on the
//! same files is told before it tries the same thing again.
//!
//! WHY: "no, don't touch the lockfile" or "use the lexer, not the parser"
//! steered one turn and was gone by the next session, and a compaction's
//! `ruled out:` line (`compact`) lived only as long as the turn that wrote
//! it. Both are the most expensive things for a model to rediscover: it only
//! learns them by being wrong again.
//!
//! Two sources, one node kind (`recall::ruled`):
//! - the user's message, as the router reads it — its optional `CORRECTS:`
//!   line (`intent::hints`), so the model decides what counts as a
//!   correction; where no router ran (a signed-in CLI leads the turn, the
//!   router is off or stumbled, plan-first) a short list of openings is the
//!   backstop ([`backstop`]);
//! - a compaction's summary, whose `ruled out:` lines are scraped as the
//!   summarizer returns them ([`remembering`]), so no caller has to know.
use crate::broker::compact::Summarize;
use crate::broker::recall::{self, SharedRecall};
use crate::broker::thread::clip_chars;

/// Chars of the user's own message kept beside the router's clause: the
/// clause says what, the message keeps the file names it was said about.
const SAID_KEPT: usize = 160;

/// Openings that say "what you did was wrong" in so many words — the
/// backstop for a turn the router never read. Matched at the very start
/// only: "no" inside a sentence is not a correction, and a missed one costs
/// a reminder, where a false one would tell every later task a falsehood.
const OPENINGS: [&str; 10] = [
    "no,",
    "no ",
    "nope",
    "don't ",
    "do not ",
    "dont ",
    "not that",
    "that's wrong",
    "thats wrong",
    "that is wrong",
];

/// Whether `task` opens like a correction, or says to use something
/// `instead` (the other way users phrase one).
pub(crate) fn backstop(task: &str) -> bool {
    let t = task.trim().to_lowercase();
    OPENINGS.iter().any(|o| t.starts_with(o)) || t.contains(" instead")
}

/// The line to remember for a correction: the router's clause when it gave
/// one, and the user's words either way.
pub(crate) fn line(clause: Option<&str>, task: &str) -> String {
    let said = clip_chars(task.trim(), SAID_KEPT);
    match clause {
        Some(c) => format!("{c} (the user: \u{201c}{said}\u{201d})"),
        None => said,
    }
}

/// Remember `task` as a correction when it is one. `clause` is the router's
/// verdict: `Some(Some(_))` it said CORRECTS, `Some(None)` it read the
/// message and said nothing (trusted: no correction), `None` no router read
/// it, so the [`backstop`] decides.
pub(crate) fn note(r: &SharedRecall, clause: Option<Option<&str>>, task: &str) {
    let what = match clause {
        Some(Some(c)) => line(Some(c), task),
        Some(None) => return,
        None if backstop(task) => line(None, task),
        None => return,
    };
    recall::ruled_out(r, &what, true);
}

/// The `ruled out:` lines of a summary, label and bullet taken off; a line
/// that says there is nothing (`none`) is not one.
pub(crate) fn ruled_lines(summary: &str) -> Vec<String> {
    summary
        .lines()
        .filter_map(|l| {
            let l = l.trim_start_matches(|c: char| c == '-' || c == '*' || c.is_whitespace());
            let head = l.get(..10)?;
            head.eq_ignore_ascii_case("ruled out:")
                .then(|| l[10..].trim().trim_end_matches('.'))
        })
        .filter(|w| !matches!(w.to_ascii_lowercase().as_str(), "" | "none" | "nothing"))
        .map(str::to_string)
        .collect()
}

/// `call`, with every `ruled out:` line of what it returns written into
/// `r` — the seam between compaction and memory, so the relay's compactor
/// and the session log's fold remember without either knowing.
pub(crate) fn remembering(call: Summarize, r: SharedRecall) -> Summarize {
    std::sync::Arc::new(move |req: &str| {
        let out = call(req)?;
        for what in ruled_lines(&out) {
            recall::ruled_out(&r, &what, false);
        }
        Ok(out)
    })
}

#[cfg(test)]
#[path = "correction_tests.rs"]
mod tests;
