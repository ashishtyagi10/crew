use serde_json::json;

use super::text_of;

#[test]
fn every_kind_of_block_says_what_it_is() {
    let r = json!({"content": [
        {"type": "text", "text": "found 2"},
        {"type": "resource_link", "uri": "file:///src/a.rs", "name": "a.rs", "description": "the parser"},
        {"type": "image", "mimeType": "image/png", "data": "A".repeat(4096)},
        {"type": "resource", "resource": {"uri": "file:///notes.md", "text": "# notes"}},
        {"type": "resource", "resource": {"uri": "file:///x.bin", "blob": "AAAA"}},
    ]});
    assert_eq!(
        text_of(&r),
        "found 2\n[a.rs: file:///src/a.rs] \u{2014} the parser\n[image image/png, 3 KB]\n\
         [file:///notes.md]\n# notes\n[file:///x.bin, 3 B binary]"
    );
}

/// A tool that answers only in structured form is read, not empty.
#[test]
fn structured_content_stands_in_when_no_block_has_words() {
    let r = json!({"content": [], "structuredContent": {"temp": 21, "unit": "C"}});
    assert_eq!(text_of(&r), r#"{"temp":21,"unit":"C"}"#);
    let both =
        json!({"content": [{"type": "text", "text": "21 C"}], "structuredContent": {"temp": 21}});
    assert_eq!(text_of(&both), "21 C", "the text block already says it");
    assert_eq!(text_of(&json!({})), "");
}
