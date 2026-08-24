//! Routing a message from a channel to an agent session, and the reply back.
//!
//! This is the step that turns "crew answers three questions from your phone" into "crew does
//! work from your phone". The pieces were already in place: the daemon owns sessions (each a
//! broker child), can write a line to one and read its output from a cursor, and every tool call
//! that session makes already passes the action gate.
//!
//! One session per channel address, kept for the life of the daemon: a conversation from a phone
//! should remember the last thing you said, and opening a fresh broker per message would throw
//! that away along with its context.
use std::collections::BTreeMap;

use crew_plugin::{PluginCommand, PluginEvent};

use super::session::Registry;

/// The broker channel name a daemon-owned session talks on. The broker echoes it back on every
/// message; nothing else depends on the value.
const CHANNEL: &str = "crew";

/// What crew says when it has taken the work but has no answer yet. A remote sender cannot see a
/// spinner, and a minute of nothing is indistinguishable from a message that never arrived.
pub(crate) const ACK: &str = "on it\u{2026}";

/// One channel address's conversation.
struct Route {
    session: String,
    /// How far through this session's output we have already read.
    cursor: usize,
}

/// Channel addresses to sessions.
#[derive(Default)]
pub(crate) struct Bridge {
    routes: BTreeMap<String, Route>,
}

impl Bridge {
    /// Hand `text` to the session for `addr`, opening one if this address has never written
    /// before. Returns the acknowledgement to send back, or an error to send back instead —
    /// either way the sender hears something.
    pub(crate) fn dispatch(
        &mut self,
        reg: &mut Registry,
        addr: &str,
        text: &str,
    ) -> Result<&'static str, String> {
        if !self.routes.contains_key(addr) {
            let id = reg
                .open(addr, None)
                .map_err(|e| format!("could not start a session: {e}"))?;
            // The broker expects a handshake before anything else; a session that skips it
            // answers nothing and looks like a hang.
            let hello =
                serde_json::to_string(&PluginCommand::Hello { v: 1 }).map_err(|e| e.to_string())?;
            reg.send(&id, &hello);
            self.routes.insert(
                addr.to_string(),
                Route {
                    session: id,
                    cursor: 0,
                },
            );
        }
        let route = self.routes.get(addr).expect("just inserted");
        let cmd = serde_json::to_string(&PluginCommand::Send {
            channel: CHANNEL.to_string(),
            text: text.to_string(),
        })
        .map_err(|e| e.to_string())?;
        match reg.send(&route.session, &cmd) {
            Some(true) => Ok(ACK),
            // The session died under us. Forget it so the next message starts a fresh one
            // rather than writing into a pipe that will never answer.
            _ => {
                self.routes.remove(addr);
                Err(
                    "that session has stopped \u{2014} say it again and I will start a new one"
                        .into(),
                )
            }
        }
    }

    /// Everything the sessions have said since the last collection, as (address, text) pairs
    /// ready to send back.
    ///
    /// Only `Message` events become replies. The broker also streams activity, stats, deltas and
    /// task lifecycle — forwarding those would turn a phone conversation into a debug log, and
    /// `Message` is the one event that carries the finished, normalized reply.
    pub(crate) fn collect(&mut self, reg: &Registry) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (addr, route) in self.routes.iter_mut() {
            let Some((lines, next, dropped)) = reg.output(&route.session, route.cursor) else {
                continue;
            };
            // A cursor that fell off the front means output was lost. Say so rather than
            // pretending the gap was never there.
            if dropped > route.cursor {
                out.push((
                    addr.clone(),
                    format!("[{} earlier line(s) were lost]", dropped - route.cursor),
                ));
            }
            route.cursor = next;
            for line in lines {
                if let Some(text) = reply_text(&line) {
                    out.push((addr.clone(), text));
                }
            }
        }
        out
    }

    /// How many addresses currently hold a session.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.routes.len()
    }
}

/// The reply text in one broker output line, if it is one.
pub(crate) fn reply_text(line: &str) -> Option<String> {
    match serde_json::from_str::<PluginEvent>(line).ok()? {
        PluginEvent::Message { text, .. } if !text.trim().is_empty() => Some(text),
        _ => None,
    }
}

#[cfg(test)]
#[path = "task_tests.rs"]
mod tests;
