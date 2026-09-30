use super::*;

fn body() -> serde_json::Value {
    serde_json::json!({"model": "gpt-5.5", "max_tokens": 4096, "messages": []})
}

#[test]
fn openai_gets_max_completion_tokens_and_no_max_tokens() {
    let b = for_endpoint("https://api.openai.com/v1/chat/completions", body());
    assert_eq!(b["max_completion_tokens"], 4096);
    assert!(b.get("max_tokens").is_none(), "{b}");
    assert_eq!(b["model"], "gpt-5.5", "the rest is untouched");
}

#[test]
fn every_other_host_keeps_max_tokens() {
    for endpoint in [
        "https://openrouter.ai/api/v1/chat/completions",
        "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions",
        "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        "https://api.deepseek.com/chat/completions",
        "https://integrate.api.nvidia.com/v1/chat/completions",
        "http://127.0.0.1:9/v1/chat/completions",
        // A lookalike host is not OpenAI.
        "https://api.openai.com.example.net/v1/chat/completions",
    ] {
        let b = for_endpoint(endpoint, body());
        assert_eq!(b["max_tokens"], 4096, "{endpoint}");
        assert!(b.get("max_completion_tokens").is_none(), "{endpoint}");
    }
}

#[test]
fn a_port_or_case_on_openai_s_host_still_counts() {
    let b = for_endpoint("https://API.OpenAI.com:443/v1/chat/completions", body());
    assert_eq!(b["max_completion_tokens"], 4096);
}
