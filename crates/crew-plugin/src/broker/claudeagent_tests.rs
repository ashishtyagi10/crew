//! Through a real child: a fake `claude` that prints the stream a line at
//! a time (the shapes `crew_hive::claudestream` documents).
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crew_hive::HiveEvent;

use super::ClaudeAgent;
use crate::broker::adapter::{Adapter, HopStream};

fn fake(script: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "crew-claudeagent-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("claude");
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    (dir, path)
}

fn agent(path: &std::path::Path, idle_floor: Duration) -> ClaudeAgent {
    ClaudeAgent {
        program: path.to_str().unwrap().into(),
        model: None,
        idle_floor,
    }
}

fn recording() -> (Arc<Mutex<Vec<String>>>, HopStream) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let (a, b, c, d) = (log.clone(), log.clone(), log.clone(), log.clone());
    let stream = HopStream {
        on_tokens: Arc::new(move |t| a.lock().unwrap().push(format!("tokens:{t}"))),
        on_text: Arc::new(move |t| b.lock().unwrap().push(format!("text:{t}"))),
        on_thought: Arc::new(move |t| c.lock().unwrap().push(format!("thought:{t}"))),
        on_tool: Arc::new(move |e| {
            d.lock().unwrap().push(match e {
                HiveEvent::ToolCall { label, args, .. } => format!("call:{label}:{args}"),
                HiveEvent::ToolResult {
                    label, ok, text, ..
                } => {
                    format!("result:{label}:{ok}:{text}")
                }
                other => format!("{other:?}"),
            })
        }),
    };
    (log, stream)
}

const STREAM: &str = concat!(
    "#!/bin/sh\n",
    "printf '%s\\n' \"$*\" > \"$0.argv\"\n",
    "echo '{\"type\":\"system\",\"subtype\":\"init\",\"tools\":[\"Read\"]}'\n",
    "echo '{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"look first\"}}}'\n",
    "echo '{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"t1\",\"name\":\"Read\",\"input\":{\"file_path\":\"/x.rs\"}}]}}'\n",
    "echo '{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"t1\",\"content\":\"fn main() {}\",\"is_error\":false}]}}'\n",
    "echo '{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"it is \"}}}'\n",
    "echo '{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"fine\"}}}'\n",
    "printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"it is fine\\n@done\",\"usage\":{\"input_tokens\":30,\"output_tokens\":9},\"total_cost_usd\":0.2}'\n",
);

/// Every lane fires in stream order — the thought, the call, its result
/// (with the call's own label), the text with its running token estimate —
/// the hop ends with the thought flush, and the reply is the `result`
/// line's text with the CLI's usage (cost 0: the plan covers it).
#[cfg(unix)]
#[test]
fn a_streaming_run_feeds_every_lane_then_settles_on_the_result() {
    let (dir, path) = fake(STREAM);
    let (log, stream) = recording();
    let (reply, usage) = agent(&path, Duration::ZERO)
        .call_with_usage_ticked("check the file", Duration::from_secs(5), &stream)
        .unwrap();
    assert_eq!(reply, "it is fine\n@done");
    assert_eq!(
        (usage.input_tokens, usage.output_tokens, usage.cost_microusd),
        (30, 9, 0)
    );
    let seen = log.lock().unwrap().clone();
    assert_eq!(
        seen,
        [
            "thought:look first",
            "call:Read:{\"file_path\":\"/x.rs\"}",
            "result:Read:true:fn main() {}",
            "text:it is ",
            "tokens:1",
            "text:fine",
            "tokens:2",
            "thought:",
        ]
    );
    let argv = std::fs::read_to_string(dir.join("claude.argv")).unwrap();
    assert_eq!(
        argv.trim(),
        "-p check the file --output-format stream-json --verbose --include-partial-messages"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The CLI's own refusal is the error, in its words; a fake that echoes
/// plain text (an older CLI, the e2e fakes) still answers.
#[cfg(unix)]
#[test]
fn a_refusal_reads_as_its_sentence_and_plain_text_still_answers() {
    let (dir, path) = fake(
        "#!/bin/sh\necho '{\"type\":\"result\",\"subtype\":\"error_during_execution\",\"is_error\":true,\"result\":\"Not logged in\"}'\n",
    );
    let err = agent(&path, Duration::ZERO)
        .call("hi", Duration::from_secs(5))
        .unwrap_err();
    assert!(err.ends_with("claude: Not logged in"), "{err}");
    std::fs::write(&path, "#!/bin/sh\necho 'claude here'\n").unwrap();
    assert_eq!(
        agent(&path, Duration::ZERO)
            .call("hi", Duration::from_secs(5))
            .unwrap(),
        "claude here"
    );
    std::fs::write(&path, "#!/bin/sh\necho 'please log in first' >&2\n").unwrap();
    let err = agent(&path, Duration::ZERO)
        .call("hi", Duration::from_secs(5))
        .unwrap_err();
    assert!(err.contains("not signed in"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The deadline is idle time, not wall time: a CLI that keeps printing
/// outlives the timeout many times over; one that goes silent is killed
/// once the window passes — and the model pin rides along as `--model`.
#[cfg(unix)]
#[test]
fn the_deadline_restarts_on_every_line_and_kills_a_silent_cli() {
    let alive = concat!(
        "#!/bin/sh\n",
        "for i in 1 2 3 4 5 6; do sleep 0.15; echo '{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"x\"}}}'; done\n",
        "echo '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"xxxxxx\"}'\n",
    );
    let (dir, path) = fake(alive);
    let reply = agent(&path, Duration::ZERO)
        .call("go", Duration::from_millis(400))
        .unwrap();
    assert_eq!(reply, "xxxxxx");

    std::fs::write(&path, "#!/bin/sh\nsleep 30\n").unwrap();
    let mut a = agent(&path, Duration::ZERO);
    a.model = Some("claude-fable-5".into());
    assert!(a
        .args("x")
        .ends_with(&["--model".to_string(), "claude-fable-5".to_string()]));
    let t0 = std::time::Instant::now();
    let err = a.call("go", Duration::from_millis(300)).unwrap_err();
    assert!(err.contains("no output for"), "{err}");
    assert!(t0.elapsed() < Duration::from_secs(5), "{:?}", t0.elapsed());
    let _ = std::fs::remove_dir_all(&dir);
}
