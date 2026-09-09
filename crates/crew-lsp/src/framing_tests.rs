use super::*;
use serde_json::json;
use std::io::Cursor;

#[test]
fn a_message_survives_the_round_trip() {
    let msg = json!({"jsonrpc": "2.0", "id": 1, "method": "x", "params": {"a": "é"}});
    let bytes = encode(&msg);
    assert!(bytes.starts_with(b"Content-Length: "), "{bytes:?}");
    let mut r = Cursor::new(bytes);
    assert_eq!(decode(&mut r).unwrap(), Some(msg));
    assert_eq!(decode(&mut r).unwrap(), None, "a clean EOF is None");
}

/// The byte count is the BODY's, in bytes not chars — an `é` is two.
#[test]
fn the_length_counts_bytes() {
    let bytes = encode(&json!("é"));
    let head = String::from_utf8_lossy(&bytes);
    assert!(head.starts_with("Content-Length: 4\r\n\r\n"), "{head}");
}

#[test]
fn headers_are_case_insensitive_and_extra_ones_are_ignored() {
    let body = r#"{"ok":true}"#;
    let wire = format!(
        "content-type: application/vscode-jsonrpc; charset=utf-8\r\nCONTENT-LENGTH: {}\r\n\r\n{body}",
        body.len()
    );
    let mut r = Cursor::new(wire.into_bytes());
    assert_eq!(decode(&mut r).unwrap(), Some(json!({"ok": true})));
}

#[test]
fn two_messages_back_to_back_come_out_in_order() {
    let mut wire = encode(&json!({"id": 1}));
    wire.extend(encode(&json!({"id": 2})));
    let mut r = Cursor::new(wire);
    assert_eq!(decode(&mut r).unwrap(), Some(json!({"id": 1})));
    assert_eq!(decode(&mut r).unwrap(), Some(json!({"id": 2})));
    assert_eq!(decode(&mut r).unwrap(), None);
}

#[test]
fn a_stream_that_ends_inside_a_frame_is_an_error_not_a_none() {
    let mut r = Cursor::new(b"Content-Length: 10\r\n\r\n{\"a\":".to_vec());
    let e = decode(&mut r).unwrap_err();
    assert!(e.contains("body"), "{e}");
    let mut r = Cursor::new(b"Content-Length: 10\r\n".to_vec());
    let e = decode(&mut r).unwrap_err();
    assert!(e.contains("inside a header"), "{e}");
}

#[test]
fn a_bad_length_or_a_non_json_body_is_an_error() {
    let mut r = Cursor::new(b"Content-Length: ten\r\n\r\n".to_vec());
    assert!(decode(&mut r).unwrap_err().contains("Content-Length"));
    let mut r = Cursor::new(b"Content-Length: 3\r\n\r\nabc".to_vec());
    assert!(decode(&mut r).unwrap_err().contains("not JSON"));
    let mut r = Cursor::new(b"X-Other: 1\r\n\r\n".to_vec());
    assert!(decode(&mut r)
        .unwrap_err()
        .contains("without a Content-Length"));
}
