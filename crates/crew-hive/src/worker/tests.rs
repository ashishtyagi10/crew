use super::*;
use crate::wire::{DepResult, RemoteReply, RemoteTask, Transport};

fn echo_handler(t: RemoteTask) -> RemoteReply {
    RemoteReply {
        task: t.task,
        output: format!("ran:{}", t.task),
        success: true,
        input_tokens: 1,
        output_tokens: 1,
    }
}

#[tokio::test]
async fn loopback_transport_dispatches_to_handler() {
    let tr = LoopbackTransport {
        handler: echo_handler,
    };
    let reply = tr
        .dispatch(RemoteTask {
            agent: 0,
            task: 9,
            prompt: "p".into(),
            model: "m".into(),
            deps: vec![],
        })
        .await
        .unwrap();
    assert_eq!(reply.output, "ran:9");
    assert!(reply.success);
}

#[test]
fn serve_stdio_processes_lines() {
    let task = RemoteTask {
        agent: 0,
        task: 3,
        prompt: "p".into(),
        model: "m".into(),
        deps: vec![DepResult {
            task: 1,
            output: "x".into(),
            success: true,
        }],
    };
    let input = format!(
        "{}\n{}\n",
        serde_json::to_string(&task).unwrap(),
        "garbage-not-json"
    );
    let mut output = Vec::new();
    serve_stdio(
        std::io::Cursor::new(input.into_bytes()),
        &mut output,
        echo_handler,
    )
    .unwrap();
    let out = String::from_utf8(output).unwrap();
    // exactly one reply line (garbage line skipped)
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 1);
    let reply: RemoteReply = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(reply.task, 3);
}
