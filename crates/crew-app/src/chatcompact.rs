//! Transcript folding: collapse a pane's message history down to the most
//! recent messages plus a marker saying how many went.
//!
//! This was `/compact`, a command. It is automatic now — `push_capped` folds
//! at `ChatPane::TRANSCRIPT_CAP` — because the cap already existed and only
//! ever dropped messages in silence, and a fold that announces itself is
//! strictly better than a manual command doing the same thing on request.
use crate::chatlayout::Message;

/// Collapse `msgs` to the last `keep`, prepended with a dim `crew` marker
/// noting how many older messages were folded away. No-op when already short.
pub(crate) fn compact_messages(msgs: Vec<Message>, keep: usize) -> Vec<Message> {
    if msgs.len() <= keep {
        return msgs;
    }
    let folded = msgs.len() - keep;
    let mut out = Vec::with_capacity(keep + 1);
    out.push(Message {
        sender: "agent smith".into(),
        text: format!(
            "(compacted {folded} earlier message{})",
            if folded == 1 { "" } else { "s" }
        ),
        ts: String::new(),
        meta: String::new(),
    });
    out.extend(msgs.into_iter().skip(folded));
    out
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
        }
    }

    fn msgs(n: usize) -> Vec<Message> {
        (0..n).map(|i| msg("user", &format!("m{i}"))).collect()
    }

    #[test]
    fn short_history_is_unchanged() {
        let v = msgs(3);
        let out = compact_messages(v, 20);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].text, "m0");
        assert_eq!(out[2].text, "m2");
    }

    #[test]
    fn long_history_folds_the_oldest_behind_a_marker() {
        let v = msgs(30);
        let out = compact_messages(v, 20);
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
        let out = compact_messages(msgs(21), 20);
        assert!(
            out[0].text.contains("1 earlier message")
                && !out[0].text.contains("1 earlier messages"),
            "got: {}",
            out[0].text
        );
        let out = compact_messages(msgs(25), 20);
        assert!(
            out[0].text.contains("5 earlier messages"),
            "got: {}",
            out[0].text
        );
    }
}
