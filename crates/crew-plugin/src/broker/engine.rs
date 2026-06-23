//! The broker engine: given a starting message, it calls the addressed agent,
//! logs the reply, and follows the routing decision — relay to a peer, reply
//! back to the sender, or finish — until the thread ends or the hop limit trips
//! the loop guard. Every hop is reported through a sink for observability.
use std::time::Duration;

use super::route::frame;
use super::{parse_routing, Envelope, Registry, Routing};

/// Why a hop was logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HopKind {
    /// A normal reply (relayed onward or bounced back to the sender).
    Reply,
    /// The agent ended the thread with `DONE`.
    Done,
    /// The hop limit was reached; the thread was dropped.
    Terminated,
    /// A launch failure, timeout, empty reply, or unknown recipient.
    Error,
}

/// One entry in the transcript: who produced it, who it is bound for, how deep
/// the relay is, why it was logged, and the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hop {
    pub from: String,
    pub to: String,
    pub hop: u32,
    pub kind: HopKind,
    pub text: String,
}

/// Routes messages between agents in a [`Registry`], with a per-call timeout and
/// a maximum hop count.
pub struct Broker {
    pub registry: Registry,
    pub max_hops: u32,
    pub timeout: Duration,
}

impl Broker {
    pub fn new(registry: Registry, max_hops: u32, timeout: Duration) -> Self {
        Self {
            registry,
            max_hops,
            timeout,
        }
    }

    /// Drive a relay that begins with a message `body` sent from `from` to `to`.
    /// Each hop is reported through `sink` as it happens. Returns when the thread
    /// ends (DONE, a reply to the originator, the hop limit, or an error).
    pub fn run(
        &self,
        from: &str,
        to: &str,
        body: &str,
        thread_id: &str,
        sink: &mut dyn FnMut(Hop),
    ) {
        let mut env = Envelope::new(from, to, thread_id, body);
        loop {
            if env.hop > self.max_hops {
                sink(self.note(
                    &env,
                    HopKind::Terminated,
                    format!("thread terminated: hop limit {} reached", self.max_hops),
                ));
                return;
            }
            let Some(agent) = self.registry.get(&env.to) else {
                sink(self.note(
                    &env,
                    HopKind::Error,
                    format!("unknown agent \"{}\"", env.to),
                ));
                return;
            };
            let prompt = frame(&env, &self.registry.peers_of(&env.to));
            let reply = match agent.call(&prompt, self.timeout) {
                Ok(r) if !r.trim().is_empty() => r,
                Ok(_) => {
                    sink(self.back(&env, HopKind::Error, "empty reply".into()));
                    return;
                }
                Err(e) => {
                    sink(self.back(&env, HopKind::Error, e));
                    return;
                }
            };
            match parse_routing(&reply) {
                Routing::Relay { to: next, body } => {
                    sink(Hop {
                        from: env.to.clone(),
                        to: next.clone(),
                        hop: env.hop,
                        kind: HopKind::Reply,
                        text: reply,
                    });
                    if self.registry.get(&next).is_none() {
                        sink(self.note(&env, HopKind::Error, format!("unknown peer \"{next}\"")));
                        return;
                    }
                    env = env.advance(env.to.clone(), next, body);
                }
                Routing::Done(answer) => {
                    sink(self.back(&env, HopKind::Done, answer));
                    return;
                }
                Routing::Reply(text) => {
                    sink(self.back(&env, HopKind::Reply, text.clone()));
                    if env.from.eq_ignore_ascii_case("user") {
                        return; // answered the originator; nothing more to relay
                    }
                    env = env.advance(env.to.clone(), env.from.clone(), text);
                }
            }
        }
    }

    /// A hop produced by the agent at `env.to`, bound back to `env.from`.
    fn back(&self, env: &Envelope, kind: HopKind, text: String) -> Hop {
        Hop {
            from: env.to.clone(),
            to: env.from.clone(),
            hop: env.hop,
            kind,
            text,
        }
    }

    /// A broker-originated note (loop guard / routing error) about `env`.
    fn note(&self, env: &Envelope, kind: HopKind, text: String) -> Hop {
        Hop {
            from: "broker".into(),
            to: env.to.clone(),
            hop: env.hop,
            kind,
            text,
        }
    }
}

impl Envelope {
    /// The next envelope one hop deeper, from `from` to `to` carrying `body`.
    fn advance(&self, from: String, to: String, body: String) -> Envelope {
        Envelope {
            from,
            to,
            thread_id: self.thread_id.clone(),
            hop: self.hop + 1,
            body,
        }
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
