//! The conversation, over two buffers. Everything a sidecar can say is said here without
//! spawning anything, which is why the process half stays as thin as it is.
use super::*;
use crate::tools::{ToolSpec, Tools};
use crate::wire::{DepResult, ToolDecl};
use std::sync::{Arc, Mutex};

/// A tool surface that records what it was asked to run.
struct Recorder(Arc<Mutex<Vec<String>>>);
impl Tools for Recorder {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.0
            .lock()
            .unwrap()
            .push(format!("{server}:{tool} {args}"));
        match tool {
            "boom" => Err("that tool refused".into()),
            _ => Ok(format!("ran {tool}")),
        }
    }
    fn specs(&self) -> Vec<ToolSpec> {
        Vec::new()
    }
}

fn task(id: u64) -> RemoteTask {
    RemoteTask {
        task: id,
        prompt: "do the thing".into(),
        model: "m".into(),
        ..RemoteTask::default()
    }
}

fn done(id: u64, output: &str) -> String {
    serde_json::to_string(&WorkerMsg::Done(RemoteReply {
        task: id,
        output: output.into(),
        success: true,
        ..RemoteReply::default()
    }))
    .unwrap()
}

/// Run `script` (the worker's lines) against `converse`, returning the reply, what crew wrote
/// back, and everything the tools were asked to run.
fn converse_with(
    script: &str,
    tools: Option<&dyn Tools>,
) -> (Result<RemoteReply, TransportError>, String, Vec<String>) {
    let deltas = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = {
        let deltas = Arc::clone(&deltas);
        move |s: &str| deltas.lock().unwrap().push(s.to_string())
    };
    let host = Host {
        tools,
        on_delta: &sink,
    };
    let mut reader = std::io::Cursor::new(script.as_bytes().to_vec());
    let mut writer: Vec<u8> = Vec::new();
    let reply = converse(task(9), &mut reader, &mut writer, &host);
    let written = String::from_utf8(writer).unwrap();
    let deltas = deltas.lock().unwrap().clone();
    (reply, written, deltas)
}

#[test]
fn the_task_goes_out_first_and_the_answer_comes_back() {
    let (reply, written, _) = converse_with(&format!("{}\n", done(9, "the answer")), None);
    assert_eq!(reply.unwrap().output, "the answer");
    let first = written.lines().next().unwrap();
    assert!(first.contains("\"kind\":\"task\""), "{first}");
    assert!(first.contains("do the thing"), "{first}");
}

#[test]
fn a_tool_call_is_run_by_crew_and_answered_on_the_same_stream() {
    // The whole reason the protocol has a turn: the worker names a tool, CREW runs it, and the
    // worker never holds a credential.
    let ran = Arc::new(Mutex::new(Vec::new()));
    let tools = Recorder(Arc::clone(&ran));
    let script = format!(
        "{}\n{}\n",
        serde_json::to_string(&WorkerMsg::Call {
            id: "c1".into(),
            tool: "sys:list_dir".into(),
            args: r#"{"path":"."}"#.into(),
        })
        .unwrap(),
        done(9, "done")
    );
    let (reply, written, _) = converse_with(&script, Some(&tools));
    assert!(reply.is_ok());
    assert_eq!(
        ran.lock().unwrap().as_slice(),
        [r#"sys:list_dir {"path":"."}"#]
    );
    let result = written.lines().nth(1).expect("a result line");
    assert!(result.contains("\"kind\":\"result\""), "{result}");
    assert!(result.contains("\"id\":\"c1\""), "{result}");
    assert!(result.contains("ran list_dir"), "{result}");
    assert!(result.contains("\"ok\":true"), "{result}");
}

#[test]
fn a_tool_that_refuses_comes_back_as_a_result_rather_than_a_failure() {
    // A refusal is information the worker can act on. Ending the task on it would make every
    // gated tool a graph failure.
    let tools = Recorder(Arc::new(Mutex::new(Vec::new())));
    let script = format!(
        "{}\n{}\n",
        serde_json::to_string(&WorkerMsg::Call {
            id: "c1".into(),
            tool: "sys:boom".into(),
            args: "{}".into(),
        })
        .unwrap(),
        done(9, "carried on")
    );
    let (reply, written, _) = converse_with(&script, Some(&tools));
    assert_eq!(reply.unwrap().output, "carried on");
    let result = written.lines().nth(1).unwrap();
    assert!(result.contains("\"ok\":false"), "{result}");
    assert!(result.contains("that tool refused"), "{result}");
}

#[test]
fn streamed_thinking_reaches_the_sink_as_it_arrives() {
    let script = format!(
        "{}\n{}\n{}\n",
        serde_json::to_string(&WorkerMsg::Delta {
            text: "thinking".into()
        })
        .unwrap(),
        serde_json::to_string(&WorkerMsg::Delta {
            text: " harder".into()
        })
        .unwrap(),
        done(9, "answer")
    );
    let (reply, _, deltas) = converse_with(&script, None);
    assert!(reply.is_ok());
    assert_eq!(deltas, ["thinking", " harder"]);
}

#[test]
fn a_worker_that_stops_without_finishing_is_an_error_not_an_empty_answer() {
    // An engine that died mid-graph must not look like one that had nothing to say.
    let (reply, _, _) = converse_with("", None);
    let e = reply.expect_err("no done line");
    assert!(e.to_string().contains("stopped before it finished"), "{e}");
}

#[test]
fn a_line_crew_cannot_read_is_stepped_over() {
    let script = format!("this is not json\n\n{}\n", done(9, "fine"));
    let (reply, _, _) = converse_with(&script, None);
    assert_eq!(reply.unwrap().output, "fine");
}

#[tokio::test]
async fn the_loopback_transport_still_answers_without_a_process() {
    let tr = LoopbackTransport {
        handler: |t: RemoteTask| RemoteReply {
            task: t.task,
            output: format!("ran:{}", t.task),
            success: true,
            ..RemoteReply::default()
        },
    };
    let sink = |_: &str| {};
    let reply = tr
        .dispatch(
            task(9),
            Host {
                tools: None,
                on_delta: &sink,
            },
        )
        .await
        .unwrap();
    assert_eq!(reply.output, "ran:9");
}

#[test]
fn serve_stdio_answers_one_task_per_line_and_skips_what_it_cannot_read() {
    let t = RemoteTask {
        task: 3,
        deps: vec![DepResult {
            task: 1,
            output: "x".into(),
            success: true,
        }],
        tools: vec![ToolDecl {
            name: "sys:run".into(),
            description: String::new(),
            input_schema: serde_json::json!({}),
        }],
        ..RemoteTask::default()
    };
    let input = format!(
        "{}\ngarbage-not-json\n",
        serde_json::to_string(&HostMsg::Task(t)).unwrap()
    );
    let mut output = Vec::new();
    serve_stdio(
        std::io::Cursor::new(input.into_bytes()),
        &mut output,
        |t: RemoteTask| RemoteReply {
            task: t.task,
            output: format!("saw {} tool(s)", t.tools.len()),
            success: true,
            ..RemoteReply::default()
        },
    )
    .unwrap();
    let lines: Vec<&str> = std::str::from_utf8(&output).unwrap().lines().collect();
    assert_eq!(lines.len(), 1, "the garbage line produced no reply");
    match serde_json::from_str::<WorkerMsg>(lines[0]).unwrap() {
        WorkerMsg::Done(r) => {
            assert_eq!(r.task, 3);
            assert_eq!(r.output, "saw 1 tool(s)", "the tools crossed the wire");
        }
        other => panic!("expected done, got {other:?}"),
    }
}
