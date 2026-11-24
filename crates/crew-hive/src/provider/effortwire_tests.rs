//! The levels a route or an agent asks for, as each wire is allowed to send
//! them: clamped to what the model takes, and never on a GPT-6 tool request,
//! where round 2's `none` (`lunaeffort`) must keep the last word.
use super::super::{AnthropicProvider, CompletionRequest};
use super::*;
use serde_json::json;

const OPENAI: &str = "https://api.openai.com/v1/chat/completions";

fn work(model: &str, effort: Effort) -> CompletionRequest {
    CompletionRequest {
        model: model.into(),
        prompt: "review this diff".into(),
        max_tokens: 4096,
        effort: Some(effort),
        ..Default::default()
    }
}

fn sent_effort(model: &str, effort: Effort) -> serde_json::Value {
    AnthropicProvider::body_if(false, &work(model, effort), true)["output_config"]["effort"].clone()
}

#[test]
fn every_level_reaches_a_model_that_takes_it() {
    assert_eq!(sent_effort("claude-opus-5-5", Effort::Max), "max");
    assert_eq!(sent_effort("claude-opus-5-5", Effort::High), "high");
    assert_eq!(sent_effort("claude-fable-5-2", Effort::Xhigh), "xhigh");
}

#[test]
fn a_level_a_model_lacks_is_brought_down_to_its_highest() {
    assert_eq!(sent_effort("claude-opus-4-5-20251101", Effort::Max), "high");
    assert_eq!(sent_effort("claude-opus-4-5", Effort::Low), "low");
    assert_eq!(sent_effort("claude-sonnet-4-6", Effort::Xhigh), "high");
    assert_eq!(sent_effort("claude-sonnet-4-6", Effort::Max), "max");
}

#[test]
fn a_written_level_is_read_and_a_mistyped_one_is_not() {
    assert_eq!(Effort::parse(" Max "), Some(Effort::Max));
    assert_eq!(Effort::parse("xhigh"), Some(Effort::Xhigh));
    assert_eq!(Effort::parse("hard"), None);
    assert_eq!(Effort::parse(""), None);
}

#[test]
fn gpt_6_without_tools_takes_up_to_high() {
    assert_eq!(
        openai_field("gpt-6.1-sol", Some(Effort::High), false),
        Some("high")
    );
    assert_eq!(
        openai_field("gpt-6-luna", Some(Effort::Low), false),
        Some("low")
    );
    assert_eq!(
        openai_field("gpt-6-astra", Some(Effort::Max), false),
        Some("high")
    );
    assert_eq!(openai_field("gpt-6.1-sol", None, false), None);
}

#[test]
fn openai_models_not_read_yet_get_nothing() {
    for model in [
        "gpt-5.5",
        "gpt-4.1",
        "gpt-60",
        "deepseek-v4-pro",
        "openai/gpt-6.1-sol",
    ] {
        assert_eq!(
            openai_field(model, Some(Effort::High), false),
            None,
            "{model}"
        );
    }
}

/// A Luna tool request routed `high` still reaches OpenAI at `none`: the
/// route's level is never put on a GPT-6 tool body, so `lunaeffort` decides.
#[test]
fn a_luna_tool_request_stays_at_none_whatever_the_route() {
    let mut body = json!({
        "model": "gpt-6-luna",
        "max_tokens": 4096,
        "messages": [],
        "tools": [{"type": "function", "function": {"name": "sys_run", "parameters": {}}}],
    });
    if let Some(e) = openai_field("gpt-6-luna", Some(Effort::High), true) {
        body["reasoning_effort"] = json!(e);
    }
    let b = super::super::tokencap::for_endpoint(OPENAI, body);
    assert_eq!(b["reasoning_effort"], "none", "{b}");
}
