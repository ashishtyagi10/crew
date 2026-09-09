use super::*;

#[test]
fn hover_contents_flatten_in_every_spelling() {
    assert_eq!(flatten_hover(&json!("plain")), "plain");
    assert_eq!(
        flatten_hover(&json!({"kind": "markdown", "value": "**bold**"})),
        "**bold**"
    );
    assert_eq!(
        flatten_hover(&json!({"language": "rust", "value": "fn main()"})),
        "```rust\nfn main()\n```"
    );
    assert_eq!(
        flatten_hover(&json!(["a", {"language": "rust", "value": "b"}, ""])),
        "a\n\n```rust\nb\n```"
    );
    assert_eq!(flatten_hover(&Value::Null), "");
    assert_eq!(flatten_hover(&json!({"kind": "markdown"})), "");
}
