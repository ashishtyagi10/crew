//! The model-facing side of `sys:spawn` / `sys:logs` / `sys:kill`: schemas,
//! argument parsing and the text an agent reads back. The processes
//! themselves live in `sysbg`. Unix-only like `sys:run` (it needs `/bin/sh`
//! and process groups); elsewhere the same plain error comes back.
use crate::mcp::McpTool;

#[cfg(unix)]
pub(crate) use super::sysbg::{reap_all, SysBg};

/// No background processes without process groups: keep the type so the
/// session compiles, and reap nothing.
#[cfg(not(unix))]
#[derive(Default)]
pub(crate) struct SysBg;
#[cfg(not(unix))]
impl SysBg {
    pub(crate) fn reap(&self) {}
}
#[cfg(not(unix))]
pub(crate) fn reap_all() {}

pub(crate) const NAMES: [&str; 3] = ["spawn", "logs", "kill"];

pub(crate) fn tools() -> Vec<McpTool> {
    let mk = |name: &str, desc: &str, props: serde_json::Value, req: &[&str]| McpTool {
        server: "sys".into(),
        name: name.into(),
        description: desc.into(),
        input_schema: serde_json::json!({"type": "object", "properties": props, "required": req}),
    };
    let wait = serde_json::json!({"type": "string", "description": "wait until this text appears in the output (or the process exits)"});
    let ms = serde_json::json!({"type": "integer", "description": "how long to wait, default 20000, at most 60000"});
    vec![
        mk(
            "spawn",
            "start a long-running command (dev server, watcher) in the background: {\"cmd\": \"npm run dev\", \"wait_for\": \"listening\"} returns an id and its first output; read it with sys:logs, stop it with sys:kill; at most 4 at once, all killed when the task ends (a program that detaches itself with setsid, as some daemons do, is not tracked)",
            serde_json::json!({"cmd": {"type": "string"}, "wait_for": wait, "timeout_ms": ms}),
            &["cmd"],
        ),
        mk(
            "logs",
            "the latest output of a background process and whether it still runs: {\"id\": 1, \"tail\": 50}; add \"wait_for\" to wait for a line to appear",
            serde_json::json!({"id": {"type": "integer"}, "tail": {"type": "integer", "description": "lines, default 50, at most 400"}, "wait_for": wait, "timeout_ms": ms}),
            &["id"],
        ),
        mk(
            "kill",
            "stop a background process and the process group it started: {\"id\": 1} (TERM, then KILL after 2 s); says its exit status",
            serde_json::json!({"id": {"type": "integer"}}),
            &["id"],
        ),
    ]
}

#[cfg(not(unix))]
pub(crate) fn call(
    _bg: &SysBg,
    tool: &str,
    _args: &str,
    _stop: &std::sync::atomic::AtomicBool,
) -> Result<String, String> {
    Err(format!("sys:{tool} is only supported on Unix platforms"))
}

#[cfg(unix)]
pub(crate) fn call(
    bg: &SysBg,
    tool: &str,
    args: &str,
    stop: &std::sync::atomic::AtomicBool,
) -> Result<String, String> {
    use std::time::Duration;
    if let Some(blocked) = super::systools::read_only_block(tool, super::systools::read_only()) {
        return Err(blocked);
    }
    let v: serde_json::Value = serde_json::from_str(args.trim())
        .map_err(|e| format!("arguments are not valid JSON: {e}"))?;
    let num = |k: &str| v.get(k).and_then(|n| n.as_u64());
    let wait_for = v
        .get("wait_for")
        .and_then(|s| s.as_str())
        .filter(|s| !s.is_empty());
    let limit = Duration::from_millis(num("timeout_ms").unwrap_or(20_000).min(60_000));
    let id = || {
        num("id")
            .map(|n| n as u32)
            .ok_or("missing integer argument \u{201c}id\u{201d}")
    };
    match tool {
        "spawn" => {
            let p = bg.spawn(super::systools::str_arg(&v, "cmd")?)?;
            // Without `wait_for`, a beat: enough for `command not found` to show.
            let how = match wait_for {
                Some(w) => p.out.wait(Some(w), limit, stop),
                None => p.out.wait(None, Duration::from_millis(300), stop),
            };
            if how == super::sysbgbuf::Waited::Stopped {
                return Ok(format!(
                    "{{\"id\": {}}}\nstopped \u{2014} {}",
                    p.id,
                    show(&p, 20)
                ));
            }
            let said = match (how, wait_for) {
                (super::sysbgbuf::Waited::Found, Some(w)) => format!("saw \u{201c}{w}\u{201d}"),
                (super::sysbgbuf::Waited::Exited, _) => "exited while waiting".into(),
                (_, Some(w)) => format!("\u{201c}{w}\u{201d} not seen in {}s", limit.as_secs()),
                _ => "started".into(),
            };
            Ok(format!("{{\"id\": {}}}\n{said}\n{}", p.id, show(&p, 20)))
        }
        "logs" => {
            let p = bg.get(id()?)?;
            if let Some(w) = wait_for {
                if p.out.wait(Some(w), limit, stop) == super::sysbgbuf::Waited::Stopped {
                    return Ok(format!("stopped \u{2014} {}", show(&p, 20)));
                }
            }
            Ok(show(&p, num("tail").unwrap_or(50).clamp(1, 400) as usize))
        }
        "kill" => {
            let id = id()?;
            Ok(format!("#{id} {}", bg.kill(id)?))
        }
        other => Err(format!("unknown background tool {other}")),
    }
}

/// Status line, then the last `tail` lines (cut to the shared result cap).
#[cfg(unix)]
fn show(p: &super::sysbgproc::Proc, tail: usize) -> String {
    let (text, dropped, exit) = p.out.read();
    let lines: Vec<&str> = text.lines().collect();
    let mut body = lines[lines.len().saturating_sub(tail)..].join("\n");
    if let Some(cut) = body.len().checked_sub(super::systools::CAP) {
        let from = (cut..=body.len())
            .find(|&i| body.is_char_boundary(i))
            .unwrap_or(0);
        body = body[from..].to_string();
    }
    let state = exit.map_or("running".to_string(), |e| {
        format!("{e} (no longer running)")
    });
    let gone = if dropped > 0 {
        format!(", {dropped} older bytes dropped")
    } else {
        String::new()
    };
    format!(
        "#{} {state}: {}\n--- last {} lines{gone}\n{body}",
        p.id,
        p.cmd,
        tail.min(lines.len())
    )
}

#[cfg(all(test, unix))]
#[path = "sysbgtools_tests.rs"]
mod tests;
