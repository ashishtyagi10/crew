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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(sender: &str, text: &str) -> Message {
        Message {
            sender: sender.into(),
            text: text.into(),
            ts: String::new(),
            meta: String::new(),
            usage: None,
        }
    }

    fn msgs(n: usize) -> Vec<Message> {
        (0..n).map(|i| msg("user", &format!("m{i}"))).collect()
    }

    #[test]
    fn short_history_is_unchanged() {
        let v = msgs(3);
        let (out, _) = compact_messages(v, 20, 0);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].text, "m0");
        assert_eq!(out[2].text, "m2");
    }

    #[test]
    fn long_history_folds_the_oldest_behind_a_marker() {
        let v = msgs(30);
        let (out, _) = compact_messages(v, 20, 0);
        assert_eq!(out.len(), 21);
        assert_eq!(out[0].sender, "agent smith");
        assert!(out[0].text.contains("compacted 10"), "got: {}", out[0].text);
        // The last message is preserved verbatim.
        assert_eq!(out[20].text, "m29");
        // The first kept (non-marker) message is the 11th original one.
        assert_eq!(out[1].text, "m10");
    }

    #[test]
    fn marker_pluralizes_the_folded_count() {
        let (out, _) = compact_messages(msgs(21), 20, 0);
        assert!(
            out[0].text.contains("1 earlier message")
                && !out[0].text.contains("1 earlier messages"),
            "got: {}",
            out[0].text
        );
        let (out, _) = compact_messages(msgs(25), 20, 0);
        assert!(
            out[0].text.contains("5 earlier messages"),
            "got: {}",
            out[0].text
        );
    }
}
