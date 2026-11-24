//! "keep" or "forget": the user's word on a proposed correction.
//!
//! WHY a word and not a command: the pane has just said, in one line, what
//! crew took as a correction and which word does what — the answer is the
//! next message, as "yes" is to a drafted commit (`intent::gate`). Exact
//! words only, and only while something is proposed: with nothing waiting,
//! "keep" is a task like any other.
//!
//! Not "yes" or "no": undo, a commit draft and a plan already answer to
//! those, and "no, …" is itself how a correction opens (`correction`).
use crate::broker::recall::{self, SharedRecall};
use crate::broker::relay::smith_answer;
use crate::broker::session::Session;
use crate::broker::thread::clip_chars;
use crate::PluginEvent;

const KEEPS: [&str; 3] = ["keep", "keep it", "keep that"];
const FORGETS: [&str; 3] = ["forget", "forget it", "forget that"];

/// Chars of the correction shown in the one line that proposes it: enough
/// to recognise, short enough to stay one line in a narrow pane.
const SHOWN: usize = 120;

/// Propose `what` and return the one line to say, or `None` when there is
/// nothing to ask (`Recall::propose_ruled`). The two words lead and the
/// quote trails: the line is broker chrome, folded when it runs long, and a
/// fold must cut the quote, never the instruction.
pub(crate) fn propose(r: &SharedRecall, what: &str) -> Option<String> {
    recall::lock(r).propose_ruled(what).then(|| {
        format!(
            "correction noted, not kept yet \u{2014} \u{201c}keep\u{201d} remembers it for later \
             work on these files, \u{201c}forget\u{201d} drops it: {}",
            clip_chars(what.trim(), SHOWN)
        )
    })
}

/// What the message says of the proposal, if it is an answer at all.
#[derive(Debug, PartialEq)]
pub(crate) enum Word {
    Keep,
    Forget,
}

pub(crate) fn word(task: &str) -> Option<Word> {
    let t = task.trim().trim_end_matches(['.', '!']).to_lowercase();
    if KEEPS.contains(&t.as_str()) {
        Some(Word::Keep)
    } else if FORGETS.contains(&t.as_str()) {
        Some(Word::Forget)
    } else {
        None
    }
}

/// The router's one call: keep or forget the proposal, or nothing — also
/// nothing when no correction is waiting, so the word falls through.
pub(crate) fn gate(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> Option<anyhow::Result<()>> {
    let w = word(task)?;
    let said = answer(&session.recall, w)?;
    Some(emit(smith_answer(said)))
}

pub(crate) fn answer(r: &SharedRecall, w: Word) -> Option<String> {
    let mut g = recall::lock(r);
    Some(match w {
        Word::Keep => format!(
            "kept: {} \u{2014} later work on these files is told it was ruled out",
            g.keep_proposed()?
        ),
        Word::Forget => format!("forgot: {}", g.drop_proposed()?),
    })
}

#[cfg(test)]
#[path = "ruledpropose_tests.rs"]
mod tests;
