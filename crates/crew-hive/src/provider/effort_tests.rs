//! The effort a request asks for, on the Anthropic wire: a judge's body
//! carries `output_config.effort`, a work turn's carries none, and a model
//! that would answer the field with an error is never sent it.
use super::super::{AnthropicProvider, CompletionRequest};
use super::*;

/// The reply judge's shape: one prompt, a small ceiling, low effort.
fn judge(model: &str) -> CompletionRequest {
    CompletionRequest {
        model: model.into(),
        prompt: "did this reply answer the question?".into(),
        max_tokens: 64,
        effort: Some(Effort::Low),
        ..Default::default()
    }
}

#[test]
fn a_judge_call_on_claude_5_asks_for_low_effort() {
    let body = AnthropicProvider::body_if(false, &judge("claude-opus-5-5"), false);
    assert_eq!(body["output_config"], serde_json::json!({"effort": "low"}));
}

#[test]
fn a_work_turn_sends_no_effort_and_keeps_the_model_default() {
    let work = CompletionRequest {
        effort: None,
        ..judge("claude-opus-5-5")
    };
    let body = AnthropicProvider::body_if(false, &work, true);
    assert!(body.get("output_config").is_none(), "{body}");
}

#[test]
fn a_model_that_rejects_effort_is_not_sent_it() {
    for model in [
        "claude-haiku-4-5",
        "claude-haiku-4-5-20251001",
        "claude-sonnet-4-5",
    ] {
        let body = AnthropicProvider::body_if(false, &judge(model), false);
        assert!(body.get("output_config").is_none(), "{model}: {body}");
    }
}

#[test]
fn the_models_that_take_effort_are_read_from_the_id() {
    for yes in [
        "claude-opus-5-5",
        "claude-sonnet-5-5",
        "claude-haiku-5-5",
        "claude-fable-5-1",
        "claude-opus-5",
        "claude-sonnet-5",
        "claude-opus-4-8",
        "claude-opus-4-5-20251101",
        "claude-sonnet-4-6",
    ] {
        assert!(anthropic_accepts(yes), "{yes}");
    }
    for no in [
        "claude-haiku-4-5",
        "claude-sonnet-4-5",
        "claude-sonnet-4-20250514",
        "claude-opus-4-1",
        "claude-3-5-sonnet-latest",
        "qwen-max",
        "",
    ] {
        assert!(!anthropic_accepts(no), "{no}");
    }
}

#[test]
fn the_levels_are_spelled_as_the_api_spells_them() {
    let all = [
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::Xhigh,
        Effort::Max,
    ];
    let words: Vec<_> = all.iter().map(|e| e.as_str()).collect();
    assert_eq!(words, ["low", "medium", "high", "xhigh", "max"]);
}
