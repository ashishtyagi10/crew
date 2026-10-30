//! A mail whose text comes in pieces around a file is read whole.
use base64::Engine;
use serde_json::json;

fn b64(s: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s)
}

/// Apple Mail with a PDF placed mid-text: the plain version, then the html
/// in two pieces with the file between them.
#[test]
fn an_inline_file_does_not_cut_the_mail_after_it() {
    let msg = json!({"payload": {"mimeType": "multipart/alternative", "parts": [
        {"mimeType": "text/plain", "body": {"data": b64("Here is the deck.\n\nCan you sign by Friday?\nAnn")}},
        {"mimeType": "multipart/mixed", "parts": [
            {"mimeType": "text/html", "body": {"data": b64("<p>Here is the deck.</p>")}},
            {"mimeType": "application/pdf", "filename": "deck.pdf", "body": {"attachmentId": "a1", "size": 9}},
            {"mimeType": "text/html", "body": {"data": b64("<p>Can you sign by Friday?</p><p>Ann</p>")}},
        ]},
    ]}});
    let (rich, files) = super::text(&msg);
    assert!(rich.body.contains("Here is the deck."), "{}", rich.body);
    assert!(
        rich.body.contains("Can you sign by Friday?"),
        "{}",
        rich.body
    );
    assert_eq!(files, vec!["deck.pdf".to_string()]);
}

/// A body and a text file sent with it: the file stays a file.
#[test]
fn a_named_text_file_is_never_joined_to_the_body() {
    let msg = json!({"payload": {"mimeType": "multipart/mixed", "parts": [
        {"mimeType": "text/plain", "body": {"data": b64("See the notes.")}},
        {"mimeType": "text/plain", "filename": "notes.txt", "body": {"data": b64("secret notes")}},
    ]}});
    let (rich, files) = super::text(&msg);
    assert_eq!(rich.body, "See the notes.");
    assert_eq!(files, vec!["notes.txt".to_string()]);
}
