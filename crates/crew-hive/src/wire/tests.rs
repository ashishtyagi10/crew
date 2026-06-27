use super::*;

#[test]
fn remote_task_serde_roundtrip() {
    let t = RemoteTask {
        agent: 1,
        task: 7,
        prompt: "do".into(),
        model: "claude-haiku-4-5".into(),
        deps: vec![DepResult {
            task: 0,
            output: "ctx".into(),
            success: true,
        }],
    };
    let line = serde_json::to_string(&t).unwrap();
    assert!(!line.contains('\n')); // single line for the wire
    assert_eq!(serde_json::from_str::<RemoteTask>(&line).unwrap(), t);
}

#[test]
fn remote_reply_serde_roundtrip() {
    let r = RemoteReply {
        task: 7,
        output: "ok".into(),
        success: true,
        input_tokens: 3,
        output_tokens: 1,
    };
    assert_eq!(
        serde_json::from_str::<RemoteReply>(&serde_json::to_string(&r).unwrap()).unwrap(),
        r
    );
}

#[test]
fn transport_is_object_safe() {
    fn _assert(_: &dyn Transport) {}
}
