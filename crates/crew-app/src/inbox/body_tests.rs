use base64::Engine;
use serde_json::json;

fn b64(s: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s)
}

#[test]
fn the_html_part_is_shown_and_the_plain_part_is_its_other_version() {
    let msg = json!({"payload": {"mimeType": "multipart/alternative", "parts": [
        {"mimeType": "text/html", "body": {"data": b64("<p>Hi <b>there</b></p>")}},
        {"mimeType": "text/plain", "body": {"data": b64("Hi there\r\n\r\n\r\n-- \r\nAnn")}},
    ]}});
    let (rich, files) = super::text(&msg);
    assert_eq!((rich.body.as_str(), files.len()), ("Hi **there**", 0));
    let alt = rich.alt.expect("the plain part");
    assert_eq!(
        (alt.body.as_str(), alt.sig.as_deref()),
        ("Hi there", Some("Ann"))
    );
}

#[test]
fn an_html_only_mail_reads_as_its_words() {
    let msg = json!({"payload": {"mimeType": "text/html", "body": {"data": b64("<html><head><style>p{}</style></head><body><p>Your order shipped.</p></body></html>")}}});
    let (rich, _) = super::text(&msg);
    assert_eq!(rich.body, "Your order shipped.");
    assert!(rich.alt.is_none());
}

#[test]
fn a_part_is_read_in_the_charset_its_header_names() {
    let bytes = [0x93u8, b'H', b'i', 0x94, b' ', 0xE9];
    let data = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let msg = json!({"payload": {"mimeType": "text/plain",
        "headers": [{"name": "Content-Type", "value": "text/plain; charset=windows-1252"}],
        "body": {"data": data}}});
    let (rich, _) = super::text(&msg);
    assert_eq!(
        (rich.body.as_str(), rich.note),
        ("\u{201c}Hi\u{201d} \u{e9}", None)
    );
    let msg = json!({"payload": {"mimeType": "text/plain",
        "headers": [{"name": "content-type", "value": "text/plain; charset=Shift_JIS"}],
        "body": {"data": b64("abc")}}});
    assert!(super::text(&msg)
        .0
        .note
        .is_some_and(|n| n.contains("shift_jis")));
}

#[test]
fn attachments_are_named_and_never_read_as_the_text() {
    let msg = json!({"payload": {"mimeType": "multipart/mixed", "parts": [
        {"mimeType": "text/plain", "filename": "notes.txt", "body": {"attachmentId": "a1"}},
        {"mimeType": "text/plain", "body": {"data": b64("See attached.")}},
        {"mimeType": "application/pdf", "filename": "invoice.pdf", "body": {"attachmentId": "a2"}},
    ]}});
    let (rich, files) = super::text(&msg);
    assert_eq!(rich.body, "See attached.");
    assert_eq!(files, ["notes.txt", "invoice.pdf"]);
}

#[test]
fn a_mail_with_no_text_says_what_it_is() {
    let msg = json!({"payload": {"mimeType": "image/png", "body": {"attachmentId": "x"}}});
    assert!(super::text(&msg).0.body.contains("image/png"));
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
    assert_eq!(super::bytes("aGk_Pz8").as_deref(), Some(&b"hi???"[..]));
    assert_eq!(super::bytes("aGk_Pz8=").as_deref(), Some(&b"hi???"[..]));
    assert_eq!(super::bytes("not base64!"), None);
}

#[test]
fn html_that_reads_as_nothing_gives_way_to_the_plain_part() {
    let msg = json!({"payload": {"mimeType": "multipart/alternative", "parts": [
        {"mimeType": "text/html", "body": {"data": b64("<img src='https://x.io/a.png'>")}},
        {"mimeType": "text/plain", "body": {"data": b64("Your statement is ready.")}},
    ]}});
    let (rich, _) = super::text(&msg);
    assert_eq!(rich.body, "Your statement is ready.");
    assert!(rich.alt.is_some());
}

#[test]
fn picture_parts_are_found_by_content_id_and_by_kind() {
    let msg = json!({"payload": {"mimeType": "multipart/related", "parts": [
        {"mimeType": "text/html", "body": {"data": b64("<img src='cid:logo@x'>")}},
        {"mimeType": "image/png", "filename": "logo.png",
         "headers": [{"name": "Content-ID", "value": "<logo@x>"}],
         "body": {"attachmentId": "A1", "size": 2048}},
        {"mimeType": "image/jpeg", "filename": "photo.jpg", "body": {"attachmentId": "A2"}},
        {"mimeType": "application/pdf", "filename": "x.pdf", "body": {"attachmentId": "A3"}},
    ]}});
    let p = super::pictures(&msg);
    assert_eq!(p.len(), 2);
    assert_eq!(
        (p[0].cid.as_deref(), p[0].att.as_deref(), p[0].size),
        (Some("logo@x"), Some("A1"), 2048)
    );
    assert_eq!(
        (p[1].cid.as_deref(), p[1].name.as_str()),
        (None, "photo.jpg")
    );
}
