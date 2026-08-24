//! Response parsing, against the shapes the Bot API actually sends.
use super::*;

fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).expect("test json")
}

#[test]
fn a_text_message_becomes_an_update() {
    let v = json(
        r#"{"ok":true,"result":[
            {"update_id":11,"message":{"message_id":1,"chat":{"id":4242,"type":"private"},"text":"hello"}}
        ]}"#,
    );
    assert_eq!(
        parse_updates(&v),
        vec![Update {
            update_id: 11,
            chat_id: 4242,
            text: "hello".into()
        }]
    );
}

/// Telegram sends plenty that is not a text message. One of them must not swallow the messages
/// on either side of it.
#[test]
fn non_message_updates_are_skipped_without_losing_their_neighbours() {
    let v = json(
        r#"{"ok":true,"result":[
            {"update_id":1,"message":{"chat":{"id":7},"text":"first"}},
            {"update_id":2,"edited_message":{"chat":{"id":7},"text":"edited"}},
            {"update_id":3,"message":{"chat":{"id":7},"photo":[]}},
            {"update_id":4,"message":{"chat":{"id":7},"text":"last"}}
        ]}"#,
    );
    let got = parse_updates(&v);
    assert_eq!(got.len(), 2, "two text messages survive: {got:?}");
    assert_eq!(got[0].text, "first");
    assert_eq!(got[1].text, "last");
    assert_eq!(got[1].update_id, 4, "the id is the real one, not a count");
}

#[test]
fn an_empty_or_malformed_response_yields_nothing_rather_than_panicking() {
    assert!(parse_updates(&json(r#"{"ok":true,"result":[]}"#)).is_empty());
    assert!(parse_updates(&json(r#"{"ok":false,"description":"nope"}"#)).is_empty());
    assert!(parse_updates(&json("{}")).is_empty());
    assert!(parse_updates(&json("[]")).is_empty());
}
