use super::*;

fn task() -> RemoteTask {
    RemoteTask {
        agent: 1,
        task: 7,
        prompt: "do".into(),
        model: "claude-haiku-4-5".into(),
        deps: vec![DepResult {
            task: 0,
            output: "ctx".into(),
            success: true,
        }],
        tools: vec![ToolDecl {
            name: "sys:run".into(),
            description: "run a shell command".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }],
        state: Some(serde_json::json!({"step": 2})),
    }
}

#[test]
fn a_task_is_one_line_and_survives_the_round_trip() {
    let t = task();
    let line = serde_json::to_string(&t).unwrap();
    assert!(!line.contains('\n'), "the wire is line-delimited");
    assert_eq!(serde_json::from_str::<RemoteTask>(&line).unwrap(), t);
}

#[test]
fn a_worker_written_against_the_first_protocol_still_reads_a_task() {
    // The bridge shipped once already. A worker that never heard of tools or state must not
    // fail to parse a task that carries them, and must still produce a task crew accepts.
    let old = r#"{"agent":1,"task":7,"prompt":"do","model":"m","deps":[]}"#;
    let t: RemoteTask = serde_json::from_str(old).unwrap();
    assert!(t.tools.is_empty());
    assert_eq!(t.state, None);
}

#[test]
fn a_reply_round_trips_with_and_without_state() {
    for state in [None, Some(serde_json::json!({"cursor": 3}))] {
        let r = RemoteReply {
            task: 7,
            output: "ok".into(),
            success: true,
            input_tokens: 3,
            output_tokens: 1,
            state,
        };
        assert_eq!(
            serde_json::from_str::<RemoteReply>(&serde_json::to_string(&r).unwrap()).unwrap(),
            r
        );
    }
}

#[test]
fn every_message_is_tagged_by_kind() {
    // The tag is the protocol: a worker reads `kind` to know what arrived, and a rename here
    // silently breaks every sidecar ever written against it.
    let task = serde_json::to_string(&HostMsg::Task(task())).unwrap();
    assert!(task.contains("\"kind\":\"task\""), "{task}");
    let result = serde_json::to_string(&HostMsg::Result {
        id: "c1".into(),
        output: "hi".into(),
        ok: true,
    })
    .unwrap();
    assert!(result.contains("\"kind\":\"result\""), "{result}");
    let delta = serde_json::to_string(&WorkerMsg::Delta {
        text: "thinking".into(),
    })
    .unwrap();
    assert!(delta.contains("\"kind\":\"delta\""), "{delta}");
    let call = serde_json::to_string(&WorkerMsg::Call {
        id: "c1".into(),
        tool: "sys:run".into(),
        args: "{}".into(),
    })
    .unwrap();
    assert!(call.contains("\"kind\":\"call\""), "{call}");
    let done = serde_json::to_string(&WorkerMsg::Done(RemoteReply::default())).unwrap();
    assert!(done.contains("\"kind\":\"done\""), "{done}");
}

#[test]
fn a_host_with_no_tools_refuses_rather_than_pretending() {
    let sink = |_: &str| {};
    let host = Host {
        tools: None,
        on_delta: &sink,
    };
    let (out, ok) = host.call("sys:run", "{}");
    assert!(!ok);
    assert!(out.contains("no tools"), "{out}");
}

#[test]
fn a_tool_name_that_is_not_server_colon_tool_is_refused_by_name() {
    let sink = |_: &str| {};
    let host = Host {
        tools: None,
        on_delta: &sink,
    };
    let (out, ok) = host.call("run", "{}");
    assert!(!ok);
    assert!(out.contains("server:tool"), "{out}");
}

#[test]
fn transport_is_object_safe() {
    fn _assert(_: &dyn Transport) {}
}
