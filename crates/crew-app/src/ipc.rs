//! The inter-pane `ask` IPC endpoint: a Unix-domain socket owned by a
//! dedicated thread (all blocking socket I/O lives here, NEVER on the winit
//! thread — see the winit-main-thread invariant). Each client connection is
//! read on its own short-lived handler thread, handed to the app as an
//! `Incoming` with a reply channel, and its verdict written back when the app
//! resolves it.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::ipc_types::{Reply, Request};

/// A request delivered to the app, with the channel its verdict is sent back on.
pub(crate) struct Incoming {
    pub req: Request,
    pub reply: Sender<Reply>,
}

/// Held by the app; drained each poll tick.
pub(crate) struct IpcHandle {
    pub rx: Receiver<Incoming>,
    path: PathBuf,
}

impl Drop for IpcHandle {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// The directory the IPC socket lives in: `$XDG_RUNTIME_DIR`, else the user
/// config dir (`~/.config/crew` / `~/Library/Application Support/crew`).
fn socket_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::config_dir().map(|d| d.join("crew")))
        .unwrap_or_else(std::env::temp_dir)
}

/// Keep only safe filename characters from an instance id, capped in length, so
/// `CREW_INSTANCE` can never escape the socket directory (no `/`, `..`, etc.).
fn sanitize_instance(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(32)
        .collect()
}

/// The socket file name. Default (`crew-ipc.sock`) unless an `instance` id is
/// given, then `crew-ipc-<id>.sock` — so multiple crew instances on one host
/// coexist and V3 federation can address each one separately. Pure in
/// `instance` so it's testable without touching the environment.
fn socket_name(instance: Option<&str>) -> String {
    match instance.map(sanitize_instance).filter(|s| !s.is_empty()) {
        Some(id) => format!("crew-ipc-{id}.sock"),
        None => "crew-ipc.sock".to_string(),
    }
}

/// The socket path for this instance: [`socket_dir`] + [`socket_name`], keyed
/// off `CREW_INSTANCE` (unset = the default instance). The `crew ask`/`crew
/// panes` client and the GUI read the same env, so they always agree.
pub(crate) fn socket_path() -> PathBuf {
    socket_dir().join(socket_name(std::env::var("CREW_INSTANCE").ok().as_deref()))
}

/// Bind `path` (reclaiming a stale socket) and spawn the listener thread.
pub(crate) fn spawn_at(path: PathBuf) -> std::io::Result<IpcHandle> {
    let _ = std::fs::remove_file(&path); // reclaim a stale socket
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let listener = UnixListener::bind(&path)?;
    let (tx, rx) = channel::<Incoming>();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let tx = tx.clone();
            std::thread::spawn(move || handle_conn(stream, tx));
        }
    });
    Ok(IpcHandle { rx, path })
}

/// Production entry: bind the default socket path.
pub(crate) fn spawn() -> std::io::Result<IpcHandle> {
    spawn_at(socket_path())
}

/// Read one JSON request line, hand it to the app, block for the verdict, and
/// write it back. Bounded so a dead app can't wedge the handler forever.
fn handle_conn(stream: UnixStream, tx: Sender<Incoming>) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(300)));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
        return;
    }
    let req: Request = match serde_json::from_str(line.trim()) {
        Ok(r) => r,
        Err(_) => return,
    };
    let (reply_tx, reply_rx) = channel::<Reply>();
    if tx
        .send(Incoming {
            req,
            reply: reply_tx,
        })
        .is_err()
    {
        return; // app gone
    }
    // Wait for the app's verdict (the app runs the adaptive wait; a hard cap
    // here is just a backstop against a wedged app).
    if let Ok(reply) = reply_rx.recv_timeout(std::time::Duration::from_secs(300)) {
        let mut s = stream;
        if let Ok(json) = serde_json::to_string(&reply) {
            let _ = s
                .write_all(json.as_bytes())
                .and_then(|_| s.write_all(b"\n"));
            let _ = s.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc_types::PaneCard;

    #[test]
    fn socket_round_trips_a_panes_request() {
        // Unique temp path (no Date/rand available; use the test thread id).
        let path = std::env::temp_dir().join(format!(
            "crew-ipc-test-{:?}.sock",
            std::thread::current().id()
        ));
        let handle = spawn_at(path.clone()).expect("bind");

        // Client: connect, send a Panes request.
        let mut client = UnixStream::connect(&path).expect("connect");
        client.write_all(b"{\"op\":\"Panes\",\"v\":1}\n").unwrap();
        client.flush().unwrap();

        // App side: receive, reply with a roster.
        let incoming = handle
            .rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert_eq!(incoming.req, Request::Panes { v: 1 });
        incoming
            .reply
            .send(Reply::Roster {
                panes: vec![PaneCard {
                    id: "p0".into(),
                    label: None,
                    kind: "terminal".into(),
                    running: None,
                    dir: None,
                    busy: false,
                }],
            })
            .unwrap();

        // Client reads the reply line.
        let mut buf = String::new();
        BufReader::new(&mut client).read_line(&mut buf).unwrap();
        let reply: Reply = serde_json::from_str(buf.trim()).unwrap();
        assert!(matches!(reply, Reply::Roster { panes } if panes.len() == 1));
        drop(handle); // unlinks the socket
    }

    #[test]
    fn socket_name_is_default_or_per_instance_and_path_safe() {
        assert_eq!(socket_name(None), "crew-ipc.sock");
        assert_eq!(socket_name(Some("alpha")), "crew-ipc-alpha.sock");
        // Empty / all-unsafe ids fall back to the shared default socket.
        assert_eq!(socket_name(Some("")), "crew-ipc.sock");
        assert_eq!(socket_name(Some("///")), "crew-ipc.sock");
        // Path-traversal attempts are stripped to a safe filename fragment.
        assert_eq!(
            socket_name(Some("../etc/passwd")),
            "crew-ipc-etcpasswd.sock"
        );
        assert_eq!(socket_name(Some("a/b\\c")), "crew-ipc-abc.sock");
    }
}
