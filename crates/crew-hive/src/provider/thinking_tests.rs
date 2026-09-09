use super::*;

const DASH: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions";
const OR: &str = "https://openrouter.ai/api/v1/chat/completions";
const NIM: &str = "https://integrate.api.nvidia.com/v1/chat/completions";

#[test]
fn dashscope_is_asked_only_on_the_streaming_request() {
    let (k, v) = opt_in_if(true, DASH, true).expect("streaming dashscope asks");
    assert_eq!((k, v), ("enable_thinking", serde_json::json!(true)));
    assert_eq!(
        opt_in_if(true, DASH, false),
        None,
        "DashScope rejects enable_thinking on a non-streamed request"
    );
}

#[test]
fn openrouter_is_asked_either_way_and_other_hosts_never() {
    let want = Some(("reasoning", serde_json::json!({"enabled": true})));
    assert_eq!(opt_in_if(true, OR, true), want);
    assert_eq!(opt_in_if(true, OR, false), want);
    assert_eq!(opt_in_if(true, NIM, true), None);
    assert_eq!(
        opt_in_if(true, "http://127.0.0.1:9/v1/chat/completions", true),
        None
    );
}

#[test]
fn crew_thinking_zero_turns_every_opt_in_off() {
    assert!(!enabled_from(Some("0")));
    assert!(enabled_from(None));
    assert!(enabled_from(Some("1")));
    assert_eq!(opt_in_if(false, DASH, true), None);
    assert_eq!(opt_in_if(false, OR, true), None);
}

#[test]
fn strip_removes_the_opt_in_and_says_whether_there_was_one() {
    let mut body = serde_json::json!({"model": "m", "reasoning": {"enabled": true}});
    assert!(strip(&mut body));
    assert!(body.get("reasoning").is_none());
    assert_eq!(body["model"], "m");
    assert!(!strip(&mut body), "nothing left to strip");
    let mut dash = serde_json::json!({"enable_thinking": true});
    assert!(strip(&mut dash));
}
