//! Transcript folding: collapse a pane's message history down to the most
//! recent messages plus a marker saying how many went.
//!
//! This was `/compact`, a command. It is automatic now — `push_capped` folds
//! at `ChatPane::TRANSCRIPT_CAP` — because the cap already existed and only
//! ever dropped messages in silence, and a fold that announces itself is
//! strictly better than a manual command doing the same thing on request.
use crate::chatlayout::Message;

/// Collapse `msgs` to the last `keep`, prepended with a marker naming how
/// many messages have been folded away IN TOTAL — `already` is the running
/// count from previous folds, and the returned count includes this one.
///
/// The running total is the whole point. Folding once per message (which is
/// what a cap does at steady state) made each fold report only what that
/// single pass dropped: after a hundred messages past the cap the marker
/// still read "compacted 2 earlier messages" while a hundred had gone.
///
/// `msgs` must not already contain a marker — the caller strips it, because
/// a marker is not one of the messages being counted.
pub(crate) fn compact_messages(
    msgs: Vec<Message>,
    keep: usize,
    already: usize,
) -> (Vec<Message>, usize) {
    if msgs.len() <= keep {
        return (msgs, already);
    }
    let dropped = msgs.len() - keep;
    let total = already + dropped;
    let mut out = Vec::with_capacity(keep + 1);
    out.push(marker(total));
    out.extend(msgs.into_iter().skip(dropped));
    (out, total)
}

/// The dim note standing in for everything folded away.
pub(crate) fn marker(total: usize) -> Message {
    Message {
        sender: "agent smith".into(),
        text: format!(
            "(compacted {total} earlier message{})",
            if total == 1 { "" } else { "s" }
        ),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

#[cfg(test)]
#[path = "chatcompact_tests.rs"]
mod tests;
