use serde_json::{json, Value};

const OPENAI: &str = "https://api.openai.com/v1/chat/completions";

fn tool_body(model: &str) -> Value {
    json!({
        "model": model,
        "max_tokens": 4096,
        "messages": [],
        "tools": [{"type": "function", "function": {"name": "sys_run", "parameters": {}}}],
    })
}

fn sent(endpoint: &str, body: Value) -> Value {
    super::super::for_endpoint(endpoint, body)
}

/// The body OpenAI's host receives for a Luna tool turn says no effort, the
/// one setting under which its page allows function calling there.
#[test]
fn a_luna_tool_request_to_openai_asks_for_no_effort() {
    let b = sent(OPENAI, tool_body("gpt-6-luna"));
    assert_eq!(b["reasoning_effort"], "none", "{b}");
    assert_eq!(
        b["max_completion_tokens"], 4096,
        "the cap rename still runs"
    );
    let dated = sent(OPENAI, tool_body("gpt-6-luna-2026-07-01"));
    assert_eq!(dated["reasoning_effort"], "none");
}

/// Every other shape is left as it was: the models that take tools at their
/// default effort, Luna answering without tools, and any gateway host.
#[test]
fn nothing_else_gets_an_effort() {
    for model in [
        "gpt-5.5",
        "gpt-4.1",
        "gpt-6.1-sol",
        "gpt-6-lunar",
        "gpt-6-luna-mini",
    ] {
        let b = sent(OPENAI, tool_body(model));
        assert!(b.get("reasoning_effort").is_none(), "{model}: {b}");
    }
    let mut plain = tool_body("gpt-6-luna");
    plain.as_object_mut().unwrap().remove("tools");
    assert!(sent(OPENAI, plain).get("reasoning_effort").is_none());
    let mut empty = tool_body("gpt-6-luna");
    empty["tools"] = json!([]);
    assert!(sent(OPENAI, empty).get("reasoning_effort").is_none());
    let gateway = sent(
        "https://openrouter.ai/api/v1/chat/completions",
        tool_body("gpt-6-luna"),
    );
    assert!(gateway.get("reasoning_effort").is_none(), "{gateway}");
}

/// An effort the caller already chose is the caller's.
#[test]
fn a_set_effort_is_kept() {
    let mut b = tool_body("gpt-6-luna");
    b["reasoning_effort"] = json!("low");
    assert_eq!(sent(OPENAI, b)["reasoning_effort"], "low");
}
