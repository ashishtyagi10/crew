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

use crew_plugin::PluginCommand;

pub(crate) use super::answers::{emitted, Emitted};
use super::answers::{parse_answer, ALLOWED, REFUSED, UNCLEAR};
use super::session::Registry;
#[cfg(test)]
pub(crate) use crew_plugin::PluginEvent;

/// The broker channel name a daemon-owned session talks on. The broker echoes it back on every
/// message; nothing else depends on the value.
const CHANNEL: &str = "crew";

/// What crew says when it has taken the work but has no answer yet. A remote sender cannot see a
/// spinner, and a minute of nothing is indistinguishable from a message that never arrived.
pub(crate) const ACK: &str = "on it\u{2026}";

/// One channel address's conversation.
struct Route {
    session: String,
    /// Where this conversation's answers go. Usually the key, and deliberately its own field:
    /// a fired intent runs in a session of its OWN (see [`Bridge::dispatch_as`]) and still has
    /// to answer on the address the person who set it reads.
    reply: String,
    /// How far through this session's output we have already read.
    cursor: usize,
    /// The approval this conversation is blocked on, if any. While it is set, the next thing
    /// the sender says is read as an ANSWER rather than a new task — the agent is stopped
    /// mid-tool-call waiting for it, and starting new work on top would leave it hanging.
    awaiting: Option<String>,
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
        // The session is for whoever is at that address, and the broker child is told so: its
        // gate must not treat a phone as a person at the keyboard.
        let who = crew_plugin::approval::Requester::Channel(addr.to_string());
        self.dispatch_as(reg, addr, addr, &who, text)
    }

    /// [`Bridge::dispatch`] with the conversation named separately from the address it answers
    /// on, and with an explicit requester.
    ///
    /// A fired intent uses this, and the separation is the security-relevant part: a trigger is
    /// the most restricted requester there is, so it must never land in the session a PERSON
    /// opened by messaging in. Sharing that session would hand a scheduled run the tier a human
    /// conversation earned — the one promotion the gate exists to prevent.
    pub(crate) fn dispatch_as(
        &mut self,
        reg: &mut Registry,
        key: &str,
        reply: &str,
        who: &crew_plugin::approval::Requester,
        text: &str,
    ) -> Result<&'static str, String> {
        let addr = key;
        if !self.routes.contains_key(addr) {
            let requester = who.to_env();
            let id = reg
                .open_for(addr, None, Some(&requester))
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
                    reply: reply.to_string(),
                    cursor: 0,
                    awaiting: None,
                },
            );
        }
        let route = self.routes.get(addr).expect("just inserted");
        // A conversation blocked on an approval hears the next message as the answer.
        if let Some(id) = route.awaiting.clone() {
            let Some(granted) = parse_answer(text) else {
                return Ok(UNCLEAR);
            };
            let cmd = serde_json::to_string(&PluginCommand::Approve { id, granted })
                .map_err(|e| e.to_string())?;
            reg.send(&route.session, &cmd);
            if let Some(r) = self.routes.get_mut(addr) {
                r.awaiting = None;
            }
            return Ok(if granted { ALLOWED } else { REFUSED });
        }
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
        for route in self.routes.values_mut() {
            let addr = route.reply.clone();
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
                match emitted(&line) {
                    Some(Emitted::Reply(text)) => out.push((addr.clone(), text)),
                    // The agent is now blocked on this. Ask, and remember which approval the
                    // next thing this address says is answering.
                    Some(Emitted::Ask { id, question }) => {
                        route.awaiting = Some(id);
                        out.push((addr.clone(), question));
                    }
                    None => {}
                }
            }
        }
        out
    }

    /// Is this conversation blocked on an approval? While it is, everything the sender says is
    /// an ANSWER, and nothing else may claim it.
    pub(crate) fn is_awaiting(&self, addr: &str) -> bool {
        self.routes.get(addr).is_some_and(|r| r.awaiting.is_some())
    }

    /// How many addresses currently hold a session.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.routes.len()
    }
}

#[cfg(test)]
#[path = "task_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "taskseam.rs"]
mod seam;
