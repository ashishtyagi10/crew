//! What a server says on its own: `notifications/tools/list_changed`.
//!
//! crew lists a server's tools once and caches them for the session, so a
//! server whose tools come and go — one that loads a plugin, logs in, or
//! opens a project — kept offering the first list: a new tool never reached
//! the model, a removed one was still offered and failed when called. A
//! server that can change says so with this notification (MCP's
//! `listChanged` capability); every message that is not the reply being
//! waited for passes [`McpClient::heard`], and the host asks
//! [`McpClient::take_tools_changed`] before serving its cache.
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use serde_json::Value;

use super::McpClient;

/// The notification a server sends when its tool list changed.
pub(super) const TOOLS_CHANGED: &str = "notifications/tools/list_changed";

impl McpClient {
    /// A message that was not the reply being waited for. A notification has
    /// a `method` and no `id`; anything else (a late reply to a request that
    /// timed out, a request from the server) is still not crew's to answer.
    pub(super) fn heard(&mut self, v: &Value) {
        let method = v.get("method").and_then(Value::as_str);
        if v.get("id").is_none() && method == Some(TOOLS_CHANGED) {
            self.tools_changed = true;
        }
    }

    /// Whether the server said its tools changed since the last listing —
    /// reading, without waiting, whatever it sent between requests — and
    /// clears the mark. Other notifications are dropped, as a wait drops
    /// them; every reply is held, in order, for the next wait.
    pub fn take_tools_changed(&mut self) -> bool {
        while let Ok(v) = self.rx.try_recv() {
            match v.get("method").is_some() && v.get("id").is_none() {
                true => self.heard(&v),
                false => self.held.push_back(v),
            }
        }
        std::mem::take(&mut self.tools_changed)
    }

    /// The next message: one held by [`Self::take_tools_changed`], else the
    /// pipe's, waiting up to `left`.
    pub(super) fn next_message(&mut self, left: Duration) -> Result<Value, RecvTimeoutError> {
        match self.held.pop_front() {
            Some(v) => Ok(v),
            None => self.rx.recv_timeout(left),
        }
    }
}
