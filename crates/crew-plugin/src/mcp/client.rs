//! One connected MCP server: a child process speaking line-delimited JSON-RPC
//! 2.0 over stdio. A reader thread feeds parsed lines into a channel; every
//! request waits for its own `id` under a hard deadline, so a hung server can
//! never block the broker. The child is killed on drop, like the plugin host.
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::config::ServerConfig;
use crew_hive::childproc::no_console_window;

/// Per-request deadline. `CREW_MCP_TIMEOUT_MS` overrides (default 30 s).
fn timeout() -> Duration {
    let ms = std::env::var("CREW_MCP_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30_000);
    Duration::from_millis(ms)
}

#[derive(Debug)]
pub struct McpClient {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Value>,
    next_id: u64,
}

impl McpClient {
    /// Launch the server and run the `initialize` handshake.
    pub fn connect(cfg: &ServerConfig) -> Result<Self, String> {
        let mut child = no_console_window(&mut Command::new(&cfg.command))
            .args(&cfg.args)
            .envs(&cfg.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to launch {}: {e}", cfg.command))?;
        let stdout = child.stdout.take().expect("stdout was piped");
        let stdin = child.stdin.take().expect("stdin was piped");
        let (tx, rx) = mpsc::channel::<Value>();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if tx.send(v).is_err() {
                        break;
                    }
                }
            }
        });
        let mut c = Self {
            child,
            stdin,
            rx,
            next_id: 0,
        };
        c.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "crew", "version": env!("CARGO_PKG_VERSION")},
            }),
        )?;
        c.notify("notifications/initialized")?;
        Ok(c)
    }

    fn send(&mut self, v: &Value) -> Result<(), String> {
        writeln!(self.stdin, "{v}").map_err(|e| format!("mcp write failed: {e}"))?;
        self.stdin
            .flush()
            .map_err(|e| format!("mcp write failed: {e}"))
    }

    fn notify(&mut self, method: &str) -> Result<(), String> {
        self.send(&json!({"jsonrpc": "2.0", "method": method}))
    }

    /// Send one request and wait for the matching response's `result`.
    /// Notifications and stray messages are skipped; an `error` member or the
    /// deadline turns into `Err`.
    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        let deadline = Instant::now() + timeout();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(format!("{method}: no response within {:?}", timeout()));
            }
            match self.rx.recv_timeout(left) {
                Ok(v) if v.get("id").and_then(Value::as_u64) == Some(id) => {
                    if let Some(err) = v.get("error") {
                        let m = err.get("message").and_then(Value::as_str).unwrap_or("?");
                        return Err(format!("{method}: {m}"));
                    }
                    return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                }
                Ok(_) => {} // a notification or someone else's reply
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(format!("{method}: server closed its pipe"));
                }
            }
        }
    }

    /// `tools/list` → `(name, one-line description)` per tool.
    pub fn tools(&mut self) -> Result<Vec<(String, String)>, String> {
        let r = self.request("tools/list", json!({}))?;
        let tools = r
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(tools
            .iter()
            .filter_map(|t| {
                let name = t.get("name")?.as_str()?.to_string();
                let desc = t
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                Some((name, desc))
            })
            .collect())
    }

    /// `tools/call` → the text content joined; `isError` results become `Err`.
    pub fn call(&mut self, tool: &str, arguments: Value) -> Result<String, String> {
        let r = self.request("tools/call", json!({"name": tool, "arguments": arguments}))?;
        let text: Vec<&str> = r
            .get("content")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter(|c| c.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|c| c.get("text").and_then(Value::as_str))
                    .collect()
            })
            .unwrap_or_default();
        let text = text.join("\n");
        if r.get("isError").and_then(Value::as_bool).unwrap_or(false) {
            Err(if text.is_empty() {
                format!("{tool} failed")
            } else {
                text
            })
        } else {
            Ok(text)
        }
    }
}

impl Drop for McpClient {
    /// Kill the server on drop — dropping a `Child` only detaches it.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
