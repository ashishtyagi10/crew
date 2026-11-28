//! Image blocks on the Anthropic wire (request JSON only; nothing is sent).
use super::build_messages;
use crate::provider::image::fake_png;
use crate::provider::{CompletionRequest, Image, ToolInvocation, ToolOutcome, Turn};
use serde_json::json;

fn shot() -> Image {
    Image::from_bytes("shot.png", &fake_png(4, 4)).unwrap()
}

fn with_result(model: &str, images: Vec<Image>) -> CompletionRequest {
    CompletionRequest {
        model: model.into(),
        prompt: "look".into(),
        turns: vec![
            Turn::Assistant {
                text: String::new(),
                calls: vec![ToolInvocation {
                    id: "c1".into(),
                    name: "sys__read_file".into(),
                    input: json!({"path": "shot.png"}),
                    bad_args: None,
                }],
            },
            Turn::ToolResults(vec![ToolOutcome {
                id: "c1".into(),
                name: "sys__read_file".into(),
                content: "image: shot.png, 4x4".into(),
                is_error: false,
                images,
            }]),
        ],
        ..Default::default()
    }
}

#[test]
fn the_opening_message_carries_the_image_before_the_text() {
    let req = CompletionRequest {
        model: "claude-sonnet-5-5".into(),
        prompt: "what is wrong here? /tmp/shot.png".into(),
        images: vec![shot()],
        ..Default::default()
    };
    let m = build_messages(&req);
    let c = &m[0]["content"];
    assert_eq!(c[0]["type"], "image");
    assert_eq!(c[0]["source"]["type"], "base64");
    assert_eq!(c[0]["source"]["media_type"], "image/png");
    assert_eq!(c[0]["source"]["data"], shot().data());
    assert_eq!(
        c[1],
        json!({"type": "text", "text": "what is wrong here? /tmp/shot.png"})
    );
}

#[test]
fn a_tool_result_image_rides_inside_the_tool_result() {
    let m = build_messages(&with_result("claude-sonnet-5-5", vec![shot()]));
    let r = &m[2]["content"][0];
    assert_eq!(r["type"], "tool_result");
    assert_eq!(r["tool_use_id"], "c1");
    assert_eq!(
        r["content"][0],
        json!({"type": "text", "text": "image: shot.png, 4x4"})
    );
    assert_eq!(r["content"][1]["type"], "image");
    assert_eq!(r["content"][1]["source"]["media_type"], "image/png");
}

#[test]
fn a_request_without_images_is_the_plain_strings_it_always_was() {
    let m = build_messages(&with_result("claude-sonnet-5-5", vec![]));
    assert_eq!(m[0]["content"], "look");
    assert_eq!(m[2]["content"][0]["content"], "image: shot.png, 4x4");
}

#[test]
fn a_text_only_model_gets_the_note_in_the_images_place() {
    let req = CompletionRequest {
        model: "deepseek-chat".into(),
        prompt: "what is wrong here?".into(),
        images: vec![shot()],
        ..Default::default()
    };
    let m = build_messages(&req);
    assert_eq!(
        m[0]["content"],
        "what is wrong here?\n\n[image omitted: shot.png, this model reads text only]"
    );
    let m = build_messages(&with_result("deepseek-chat", vec![shot()]));
    assert_eq!(
        m[2]["content"][0]["content"],
        "image: shot.png, 4x4\n\n[image omitted: shot.png, this model reads text only]"
    );
}

#[test]
fn a_new_image_changes_only_the_newest_turn() {
    // The prefix the prompt cache keys on: every message before the newest is
    // identical with or without the newest turn's picture.
    let (a, b) = (
        build_messages(&with_result("claude-sonnet-5-5", vec![])),
        build_messages(&with_result("claude-sonnet-5-5", vec![shot()])),
    );
    assert_eq!(a[..2], b[..2]);
    assert_ne!(a[2], b[2]);
}

#[test]
fn anthropic_wire_carries_exactly_twenty_with_notes_in_place() {
    let m = build_messages(&crate::provider::imagecap::tests::request(
        "claude-sonnet-5-5",
        4,
    ));
    let v = serde_json::Value::Array(m.clone());
    assert_eq!(crate::provider::imagecap::tests::count(&v, "image"), 20);
    let text = v.to_string();
    for i in 0..5 {
        assert!(text.contains(&crate::provider::imagecap::tests::note(&format!("r0_{i}"))));
    }
    assert!(!text.contains("image dropped: r1_0"));
    // the note sits in round 0's tool_result, where its pictures were
    assert!(m[2]["content"][0]["content"].to_string().contains("r0_0"));
}
