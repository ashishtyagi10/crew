//! The documents a [`Client`] has told its server about, and what the
//! server says back on its own initiative. Split from `client` along the
//! line the protocol draws: `client` is request/response, this is the
//! notifications flowing both ways.
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::demux::Notification;
use crate::types::Diagnostic;
use crate::Client;

impl Client {
    pub fn is_open(&self, uri: &str) -> bool {
        self.open.contains(uri)
    }

    /// Tell the server about a document. The text is what the caller has —
    /// the disk's, for crew — and version 1 forever, since crew never edits.
    pub fn did_open(&mut self, uri: &str, language_id: &str, text: &str) -> Result<(), String> {
        self.notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": language_id, "version": 1, "text": text}}),
        )?;
        self.open.insert(uri.to_string());
        Ok(())
    }

    pub fn did_close(&mut self, uri: &str) -> Result<(), String> {
        self.open.remove(uri);
        self.notify(
            "textDocument/didClose",
            json!({"textDocument": {"uri": uri}}),
        )
    }

    pub fn try_notification(&self) -> Option<Notification> {
        self.notif.try_recv().ok()
    }

    pub fn wait_notification(&self, timeout: Duration) -> Option<Notification> {
        self.notif.recv_timeout(timeout).ok()
    }

    /// Wait for the next `publishDiagnostics` about `uri`. Other files'
    /// diagnostics and every other notification are consumed on the way.
    pub fn wait_diagnostics(&self, uri: &str, timeout: Duration) -> Option<Vec<Diagnostic>> {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let n = self.wait_notification(left)?;
            if n.method == "textDocument/publishDiagnostics" {
                if let Some((for_uri, list)) = Diagnostic::parse_publish(&n.params) {
                    if for_uri == uri {
                        return Some(list);
                    }
                }
            }
        }
    }

    /// The polite exit: `shutdown`, briefly waited for, then `exit`. Drop
    /// does the impolite one either way.
    pub fn shutdown(&mut self) {
        let _ = self.request("shutdown", Value::Null, Duration::from_secs(2));
        let _ = self.notify("exit", Value::Null);
    }
}
