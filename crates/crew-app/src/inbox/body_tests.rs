use base64::Engine;
use serde_json::json;

fn b64(s: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s)
}

#[test]
fn the_plain_part_wins_over_the_html_one() {
    let msg = json!({"payload": {"mimeType": "multipart/alternative", "parts": [
        {"mimeType": "text/html", "body": {"data": b64("<p>Hi <b>there</b></p>")}},
        {"mimeType": "text/plain", "body": {"data": b64("Hi there\r\n\r\n\r\n-- Ann")}},
    ]}});
    assert_eq!(
        super::text(&msg),
        ("Hi there\n\n-- Ann".to_string(), vec![])
    );
}

#[test]
fn an_html_only_mail_is_read_as_text() {
    let msg = json!({"payload": {"mimeType": "text/html", "body": {"data": b64("<html><head><style>p{}</style></head><body><p>Your order shipped.</p></body></html>")}}});
    let (text, _) = super::text(&msg);
    assert!(text.contains("Your order shipped."), "{text:?}");
    assert!(!text.contains('<') && !text.contains("p{}"), "{text:?}");
}

#[test]
fn attachments_are_named_and_never_read_as_the_text() {
    let msg = json!({"payload": {"mimeType": "multipart/mixed", "parts": [
        {"mimeType": "text/plain", "filename": "notes.txt", "body": {"attachmentId": "a1"}},
        {"mimeType": "text/plain", "body": {"data": b64("See attached.")}},
        {"mimeType": "application/pdf", "filename": "invoice.pdf", "body": {"attachmentId": "a2"}},
    ]}});
    assert_eq!(
        super::text(&msg),
        (
            "See attached.".into(),
            vec!["notes.txt".into(), "invoice.pdf".into()]
        )
    );
}

#[test]
fn a_mail_with_no_text_says_what_it_is() {
    let msg = json!({"payload": {"mimeType": "image/png", "body": {"attachmentId": "x"}}});
    assert!(super::text(&msg).0.contains("image/png"));
}

#[test]
fn escape_sequences_never_reach_the_grid() {
    assert_eq!(
        super::clean("red \u{1b}[31mtext\u{7}\tok"),
        "red [31mtext\tok"
    );
}

#[test]
fn padded_and_unpadded_base64url_both_decode() {
    assert_eq!(super::decode("aGk_Pz8").as_deref(), Some("hi???"));
    assert_eq!(super::decode("aGk_Pz8=").as_deref(), Some("hi???"));
    assert_eq!(super::decode("not base64!"), None);
}
