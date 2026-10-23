use super::*;

const DASH: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions";
const OR: &str = "https://openrouter.ai/api/v1/chat/completions";
const NIM: &str = "https://integrate.api.nvidia.com/v1/chat/completions";

#[test]
fn dashscope_thinks_only_on_the_streaming_request() {
    assert_eq!(
        opt_in_if(true, DASH, true, 512),
        vec![
            ("enable_thinking", serde_json::json!(true)),
            ("thinking_budget", serde_json::json!(512)),
        ],
        "asked, and bounded"
    );
    // A one-shot call (judge, router, commit message) is told not to think:
    // qwen3.8-max otherwise thinks on it unbounded, unseen, and late.
    let off = vec![("enable_thinking", serde_json::json!(false))];
    assert_eq!(opt_in_if(true, DASH, false, 512), off);
    assert_eq!(
        opt_in_if(false, DASH, false, 512),
        off,
        "CREW_THINKING=0 too"
    );
    // …and a model that only thinks, and so refuses `false`, gets the one
    // retry without it (`openai_http`).
    let mut body = serde_json::json!({"model": "m", "enable_thinking": false});
    assert!(strip(&mut body));
    assert!(body.get("enable_thinking").is_none());
}

#[test]
fn openrouter_is_asked_either_way_and_other_hosts_never() {
    let want = vec![(
        "reasoning",
        serde_json::json!({"enabled": true, "max_tokens": 300}),
    )];
    assert_eq!(opt_in_if(true, OR, true, 300), want);
    assert_eq!(opt_in_if(true, OR, false, 300), want);
    assert!(opt_in_if(true, NIM, true, 300).is_empty());
    assert!(opt_in_if(true, "http://127.0.0.1:9/v1/chat/completions", true, 300).is_empty());
}

#[test]
fn crew_thinking_zero_turns_every_opt_in_off() {
    assert!(!enabled_from(Some("0")));
    assert!(enabled_from(None));
    assert!(enabled_from(Some("1")));
    assert!(opt_in_if(false, DASH, true, 512).is_empty());
    assert_eq!(
        opt_in_if(false, DASH, false, 512).len(),
        1,
        "a no, not an opt-in"
    );
    assert!(opt_in_if(false, OR, true, 512).is_empty());
}

#[test]
fn strip_removes_the_opt_in_and_says_whether_there_was_one() {
    let mut body = serde_json::json!({"model": "m", "reasoning": {"enabled": true}});
    assert!(strip(&mut body));
    assert!(body.get("reasoning").is_none());
    assert_eq!(body["model"], "m");
    assert!(!strip(&mut body), "nothing left to strip");
    let mut dash = serde_json::json!({"enable_thinking": true, "thinking_budget": 512});
    assert!(strip(&mut dash));
    assert!(
        dash.get("thinking_budget").is_none(),
        "the budget goes with the opt-in"
    );
}
