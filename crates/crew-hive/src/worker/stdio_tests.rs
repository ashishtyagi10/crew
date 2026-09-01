//! The process half. A sidecar written in another language is the point of it, so the end-to-end
//! test drives a real one — and skips itself, loudly, on a machine with no Python, which is the
//! same machine the feature is designed to behave normally on.
use super::*;

#[test]
fn a_command_is_read_the_way_a_shell_would_read_it() {
    assert_eq!(
        parse_command("python3 -m crew_langgraph"),
        Some(("python3".into(), vec!["-m".into(), "crew_langgraph".into()]))
    );
    assert_eq!(parse_command("worker"), Some(("worker".into(), vec![])));
}

#[test]
fn no_command_is_no_sidecar_rather_than_an_empty_one() {
    // The default. A blank config key must not spawn `""` and fail obscurely.
    assert_eq!(parse_command(""), None);
    assert_eq!(parse_command("   "), None);
}

#[test]
fn a_program_that_is_not_there_is_probed_as_absent() {
    assert!(!probe("crew-no-such-program-anywhere"));
}

#[test]
fn a_program_on_the_path_is_probed_as_present() {
    // Something that exists on every machine crew builds on.
    let known = if cfg!(windows) { "cmd" } else { "sh" };
    assert!(probe(known), "{known} should be on PATH");
}

/// A sidecar in twelve lines of Python: reads a task, streams a delta, asks crew to run a tool,
/// and answers with what the tool said. It exercises the whole protocol from the other side.
const SIDECAR: &str = r#"
import json, sys
for line in sys.stdin:
    msg = json.loads(line)
    if msg["kind"] == "task":
        print(json.dumps({"kind": "delta", "text": "working"}), flush=True)
        print(json.dumps({"kind": "call", "id": "c1", "tool": "sys:list_dir",
                          "args": "{}"}), flush=True)
    elif msg["kind"] == "result":
        print(json.dumps({"kind": "done", "task": 9, "output": "tool said: " + msg["output"],
                          "success": True, "input_tokens": 0, "output_tokens": 0,
                          "state": {"step": 1}}), flush=True)
"#;

#[tokio::test]
async fn a_real_sidecar_process_streams_calls_a_tool_and_answers() {
    let Some(python) = ["python3", "python"].into_iter().find(|p| probe(p)) else {
        eprintln!("skipped: no python on this machine (the feature is opt-in for exactly that)");
        return;
    };
    let dir = std::env::temp_dir().join(format!("crew-sidecar-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("sidecar.py");
    std::fs::write(&script, SIDECAR).unwrap();

    struct Fake;
    impl crate::tools::Tools for Fake {
        fn hint(&self) -> String {
            String::new()
        }
        fn call(&self, server: &str, tool: &str, _args: &str) -> Result<String, String> {
            Ok(format!("{server}:{tool} ran here, not there"))
        }
    }
    let deltas = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let sink = {
        let deltas = std::sync::Arc::clone(&deltas);
        move |s: &str| deltas.lock().unwrap().push(s.to_string())
    };
    let tools = Fake;
    let tr = StdioTransport::spawn(python, &[script.to_string_lossy().into_owned()]).unwrap();
    let reply = tr
        .dispatch(
            RemoteTask {
                task: 9,
                prompt: "do it".into(),
                ..RemoteTask::default()
            },
            Host {
                tools: Some(&tools),
                on_delta: &sink,
            },
        )
        .await
        .expect("the sidecar answered");
    assert_eq!(reply.output, "tool said: sys:list_dir ran here, not there");
    assert_eq!(
        reply.state,
        Some(serde_json::json!({"step": 1})),
        "state comes back opaque, for the next task"
    );
    assert_eq!(deltas.lock().unwrap().as_slice(), ["working"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_sidecar_that_exits_says_so_by_name() {
    let Some(python) = ["python3", "python"].into_iter().find(|p| probe(p)) else {
        return;
    };
    // Reads nothing, answers nothing, exits — the shape of a sidecar with a syntax error.
    let tr = StdioTransport::spawn(python, &["-c".into(), "pass".into()]).unwrap();
    let sink = |_: &str| {};
    let err = tr
        .dispatch(
            RemoteTask::default(),
            Host {
                tools: None,
                on_delta: &sink,
            },
        )
        .await
        .expect_err("a worker that exits cannot answer");
    let msg = err.to_string();
    assert!(msg.contains("exited"), "{msg}");
    assert!(msg.contains(python), "and it names what was spawned: {msg}");
}
