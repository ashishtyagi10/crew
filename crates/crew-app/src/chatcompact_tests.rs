use super::*;

fn msg(sender: &str, text: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
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
        out[0].text.contains("1 earlier message") && !out[0].text.contains("1 earlier messages"),
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
