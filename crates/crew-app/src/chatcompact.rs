//! `/compact`: a pane-local composer command (claude-code-style) that
//! collapses the crew pane's message history down to the most recent
//! messages plus a marker, decluttering a long session. Handled app-side,
//! like `/export`/`/theme` — the broker never sees it.
use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// Default number of most-recent messages a bare `/compact` keeps.
const DEFAULT_KEEP: usize = 20;

/// Collapse `msgs` to the last `keep`, prepended with a dim `crew` marker
/// noting how many older messages were folded away. No-op when already short.
pub(crate) fn compact_messages(msgs: Vec<Message>, keep: usize) -> Vec<Message> {
    if msgs.len() <= keep {
        return msgs;
    }
    let folded = msgs.len() - keep;
    let mut out = Vec::with_capacity(keep + 1);
    out.push(Message {
        sender: "crew".into(),
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

/// Parse the `/compact [n]` argument: `n` when it parses as a count, else the
/// default (also on a missing or invalid argument).
fn parse_keep(arg: &str) -> usize {
    arg.trim().parse().unwrap_or(DEFAULT_KEEP)
}

/// Intercept composer submissions the pane answers locally. Returns `true`
/// when `text` was consumed (nothing should be sent to the broker).
pub(crate) fn intercept(pane: &mut ChatPane, text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed != "/compact" && !trimmed.starts_with("/compact ") {
        return false;
    }
    let arg = trimmed.strip_prefix("/compact").unwrap_or("");
    let keep = parse_keep(arg);
    pane.messages = compact_messages(std::mem::take(&mut pane.messages), keep);
    pane.scroll = 0;
    pane.unread = 0;
    true
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
        assert_eq!(out[0].sender, "crew");
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

    #[test]
    fn parse_keep_falls_back_to_default_on_missing_or_invalid() {
        assert_eq!(parse_keep(""), DEFAULT_KEEP);
        assert_eq!(parse_keep(" "), DEFAULT_KEEP);
        assert_eq!(parse_keep("nope"), DEFAULT_KEEP);
        assert_eq!(parse_keep(" 5 "), 5);
    }
}
