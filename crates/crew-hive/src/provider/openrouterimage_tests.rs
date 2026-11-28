//! Image parts on the OpenAI wire (request JSON only; nothing is sent).
use super::{build_body, build_messages};
use crate::provider::image::fake_png;
use crate::provider::{CompletionRequest, Image, ToolInvocation, ToolOutcome, Turn};
use serde_json::json;

fn shot(name: &str) -> Image {
    Image::from_bytes(name, &fake_png(4, 4)).unwrap()
}

fn out(id: &str, images: Vec<Image>) -> ToolOutcome {
    ToolOutcome {
        id: id.into(),
        name: "sys__read_file".into(),
        content: "image: a.png, 4x4".into(),
        is_error: false,
        images,
    }
}

fn with_results(model: &str, results: Vec<ToolOutcome>) -> CompletionRequest {
    let calls = results
        .iter()
        .map(|r| ToolInvocation {
            id: r.id.clone(),
            name: r.name.clone(),
            input: json!({}),
            bad_args: None,
        })
        .collect();
    CompletionRequest {
        model: model.into(),
        prompt: "look".into(),
        turns: vec![
            Turn::Assistant {
                text: String::new(),
                calls,
            },
            Turn::ToolResults(results),
        ],
        ..Default::default()
    }
}

#[test]
fn the_opening_message_carries_an_image_url_part() {
    let img = shot("a.png");
    let req = CompletionRequest {
        model: "gpt-6".into(),
        prompt: "what is wrong here?".into(),
        images: vec![img.clone()],
        ..Default::default()
    };
    let c = &build_messages(&req)[0]["content"];
    assert_eq!(c[0], json!({"type": "text", "text": "what is wrong here?"}));
    assert_eq!(c[1]["type"], "image_url");
    assert_eq!(c[1]["image_url"]["url"], img.data_url());
}

#[test]
fn tool_messages_stay_text_and_the_pictures_follow_the_whole_batch() {
    let req = with_results(
        "gpt-6",
        vec![
            out("c1", vec![shot("a.png")]),
            out("c2", vec![shot("b.png")]),
        ],
    );
    let m = build_messages(&req);
    // user, assistant, tool, tool, user: the pictures never sit between the tools.
    let roles: Vec<&str> = m.iter().map(|x| x["role"].as_str().unwrap()).collect();
    assert_eq!(roles, ["user", "assistant", "tool", "tool", "user"]);
    assert_eq!(m[2]["content"], "image: a.png, 4x4");
    let parts = m[4]["content"].as_array().unwrap();
    assert_eq!(parts[0]["text"], "Image from sys__read_file: a.png");
    assert_eq!(parts[1]["type"], "image_url");
    assert_eq!(parts[2]["text"], "Image from sys__read_file: b.png");
}

#[test]
fn no_image_means_the_same_messages_as_before() {
    let m = build_messages(&with_results("gpt-6", vec![out("c1", vec![])]));
    assert_eq!(m.len(), 3);
    assert_eq!(m[0]["content"], "look");
}

#[test]
fn a_text_only_model_gets_the_note() {
    let m = build_messages(&with_results(
        "qwen-max",
        vec![out("c1", vec![shot("a.png")])],
    ));
    assert_eq!(
        m.len(),
        3,
        "no follow-up message for a model that cannot see it"
    );
    assert_eq!(
        m[2]["content"],
        "image: a.png, 4x4\n\n[image omitted: a.png, this model reads text only]"
    );
}

#[test]
fn a_text_only_fallback_is_rendered_its_own_request() {
    let req = CompletionRequest {
        model: "gpt-6".into(),
        prompt: "q".into(),
        images: vec![shot("a.png")],
        ..Default::default()
    };
    let messages = build_messages(&req);
    let seen = build_body("gpt-6", &req, &messages, false, vec![]);
    assert_eq!(seen["messages"][0]["content"][1]["type"], "image_url");
    let fell = build_body("deepseek-chat", &req, &messages, false, vec![]);
    assert_eq!(
        fell["messages"][0]["content"],
        "q\n\n[image omitted: a.png, this model reads text only]"
    );
}

#[test]
fn openrouter_wire_carries_the_capped_set_with_notes_in_place() {
    let m = build_messages(&crate::provider::imagecap::tests::request("gpt-4o", 4));
    let v = serde_json::Value::Array(m.clone());
    assert_eq!(crate::provider::imagecap::tests::count(&v, "image_url"), 19);
    let tool = m.iter().find(|x| x["role"] == "tool").unwrap();
    assert!(tool["content"]
        .as_str()
        .unwrap()
        .contains(&crate::provider::imagecap::tests::note("r0_0")));
    assert!(!v.to_string().contains("image dropped: r1_1"));
}
