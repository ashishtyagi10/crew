//! One running language server: the process, its framed stdio, and the
//! handshake. The documents it has been told about live in [`crate::docs`]. Every wait has a deadline, so a server
//! that hangs (or never indexed) costs a timeout, never a frozen caller.
use std::collections::BTreeSet;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::demux::{self, Incoming, Notification};
use crate::framing::encode;
use crew_hive::childproc::no_console_window;

/// How long to wait before re-asking a server that said "not yet".
const RETRY_PAUSE: Duration = Duration::from_millis(200);

/// A language server crew has started.
pub struct Client {
    child: Child,
    /// Shared with the reader thread, which answers server-to-client
    /// requests on our behalf (see [`demux::reply_to`]).
    stdin: Arc<Mutex<ChildStdin>>,
    resp: Receiver<Value>,
    pub(crate) notif: Receiver<Notification>,
    next_id: u64,
    lang: String,
    root: PathBuf,
    pub(crate) open: BTreeSet<String>,
}

fn write_msg(stdin: &Mutex<ChildStdin>, msg: &Value) -> Result<(), String> {
    let mut w = stdin.lock().unwrap_or_else(|e| e.into_inner());
    w.write_all(&encode(msg))
        .and_then(|_| w.flush())
        .map_err(|e| format!("lsp write failed: {e}"))
}

impl Client {
    /// Start `cmd` in `root` and wire its stdio. No handshake yet — see
    /// [`Client::initialize`] — so a caller can bound the two separately.
    pub fn spawn(cmd: &str, args: &[String], lang: &str, root: &Path) -> Result<Self, String> {
        let mut command = Command::new(cmd);
        no_console_window(&mut command);
        command
            .args(args)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // Its own process group, so `drop` takes the whole tree: rust-analyzer
        // runs `cargo check` underneath itself, and that is the expensive
        // thing to leave behind.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("failed to launch {cmd}: {e}"))?;
        let stdout = child.stdout.take().expect("stdout was piped");
        let stdin = Arc::new(Mutex::new(child.stdin.take().expect("stdin was piped")));
        let (resp_tx, resp) = mpsc::channel();
        let (notif_tx, notif) = mpsc::channel();
        let reply_via = Arc::clone(&stdin);
        std::thread::spawn(move || {
            demux::pump(BufReader::new(stdout), |m| match m {
                Incoming::Response(v) => resp_tx.send(v).is_ok(),
                Incoming::Notification(n) => notif_tx.send(n).is_ok(),
                Incoming::ServerRequest { id, method, params } => {
                    write_msg(&reply_via, &demux::reply_to(&method, id, &params)).is_ok()
                }
            });
        });
        crate::running::register(lang, root);
        Ok(Self {
            child,
            stdin,
            resp,
            notif,
            next_id: 0,
            lang: lang.to_string(),
            root: root.to_path_buf(),
            open: BTreeSet::new(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The `initialize` / `initialized` handshake, declaring only what crew
    /// reads: diagnostics pushed to it and markdown hovers. Declaring less
    /// is what keeps the server from asking for things (`window/workDone
    /// Progress/create`, `client/registerCapability`) that need answering.
    pub fn initialize(&mut self, timeout: Duration) -> Result<Value, String> {
        let root_uri = crate::uri::from_path(&self.root);
        let params = json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "workspaceFolders": [{"uri": root_uri, "name": self.root.file_name()
                .map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()}],
            "clientInfo": {"name": "crew", "version": env!("CARGO_PKG_VERSION")},
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {"relatedInformation": false},
                    "hover": {"contentFormat": ["markdown", "plaintext"]},
                    "definition": {"linkSupport": true},
                },
            },
        });
        let r = self.request("initialize", params, timeout)?;
        self.notify("initialized", json!({}))?;
        Ok(r)
    }

    /// Send one request and wait for its reply. Replies to anything else
    /// are dropped — there is only ever one request in flight per client.
    ///
    /// A server still indexing answers `ContentModified` (-32801) or
    /// `ServerCancelled` (-32802) rather than waiting; both mean "ask again",
    /// so the request is re-sent until the deadline rather than surfaced as
    /// a failure the caller would only retry anyway.
    pub fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.request_once(method, params.clone(), left) {
                Err((Some(-32801 | -32802), _)) if left > RETRY_PAUSE => {
                    std::thread::sleep(RETRY_PAUSE);
                }
                Err((_, msg)) => return Err(msg),
                Ok(v) => return Ok(v),
            }
        }
    }

    /// One send and one wait; the error carries the JSON-RPC code, if any.
    fn request_once(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, (Option<i64>, String)> {
        self.next_id += 1;
        let id = self.next_id;
        write_msg(
            &self.stdin,
            &json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
        )
        .map_err(|e| (None, e))?;
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err((None, format!("{method}: no reply within {timeout:?}")));
            }
            match self.resp.recv_timeout(left) {
                Ok(v) if v.get("id").and_then(Value::as_u64) == Some(id) => {
                    if let Some(err) = v.get("error") {
                        let m = err.get("message").and_then(Value::as_str).unwrap_or("?");
                        let code = err.get("code").and_then(Value::as_i64);
                        return Err((code, format!("{method}: {m}")));
                    }
                    return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                }
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err((None, format!("{method}: the server closed its pipe")));
                }
            }
        }
    }

    pub fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        write_msg(
            &self.stdin,
            &json!({"jsonrpc": "2.0", "method": method, "params": params}),
        )
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        crate::running::unregister(&self.lang, &self.root);
        #[cfg(unix)]
        unsafe {
            libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
