//! The broker engine: given a starting message, it calls the addressed agent,
//! logs the reply, and follows the routing decision — relay to a peer, reply
//! back to the sender, or finish — until the thread ends or the hop limit trips
//! the loop guard. Every hop is reported through a sink for observability.
use std::time::Duration;

use super::route::{clip, frame};
use super::{parse_routing, Envelope, Registry, Routing};

/// Why a hop was logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HopKind {
    /// About to call an agent (progress note shown before the call); `to` = it.
    Dialing,
    /// A normal reply (relayed onward or bounced back to the sender).
    Reply,
    /// The agent ended the thread with `DONE`.
    Done,
    /// The hop limit was reached; the thread was dropped.
    Terminated,
    /// A launch failure, timeout, empty reply, or unknown recipient.
    Error,
}

/// One transcript entry: who produced it, who it's bound for, depth, kind, text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hop {
    pub from: String,
    pub to: String,
    pub hop: u32,
    pub kind: HopKind,
    pub text: String,
}

/// Approximate cost of a relay: agent calls made and ~tokens (chars / 4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunStats {
    pub exchanges: u32,
    pub approx_tokens: usize,
}

/// Routes messages between agents in a [`Registry`], with a per-call timeout, a
/// maximum hop count, and an approximate token budget (0 = unlimited).
pub struct Broker {
    pub registry: Registry,
    pub max_hops: u32,
    pub timeout: Duration,
    pub token_budget: usize,
}

impl Broker {
    pub fn new(registry: Registry, max_hops: u32, timeout: Duration) -> Self {
        Self {
            registry,
            max_hops,
            timeout,
            token_budget: 0,
        }
    }

    /// Cap a thread's approximate token spend (0 = unlimited).
    pub fn with_budget(mut self, tokens: usize) -> Self {
        self.token_budget = tokens;
        self
    }

    /// Drive a relay from `from` to `to`; hops stream through `sink`.
    pub fn run(
        &self,
        from: &str,
        to: &str,
        body: &str,
        thread_id: &str,
        sink: &mut dyn FnMut(Hop),
    ) -> RunStats {
        let task = body.to_string();
        let mut transcript: Vec<String> = Vec::new();
        let mut stats = RunStats::default();
        let mut env = Envelope::new(from, to, thread_id, body);
        loop {
            if env.hop > self.max_hops {
                sink(self.note(
                    &env,
                    HopKind::Terminated,
                    format!("thread terminated: hop limit {} reached", self.max_hops),
                ));
                return stats;
            }
            let Some(agent) = self.registry.get(&env.to) else {
                sink(self.note(
                    &env,
                    HopKind::Error,
                    format!("unknown agent \"{}\"", env.to),
                ));
                return stats;
            };
            let peers = self.registry.peers_of(&env.to);
            let prompt = frame(&env, &peers, &task, &tail(&transcript));
            sink(self.note(&env, HopKind::Dialing, String::new()));
            let reply = match agent.call(&prompt, self.timeout) {
                Ok(r) if !r.trim().is_empty() => r,
                Ok(_) => {
                    sink(self.back(&env, HopKind::Error, "empty reply".into()));
                    return stats;
                }
                Err(e) => {
                    sink(self.back(&env, HopKind::Error, e));
                    return stats;
                }
            };
            stats.exchanges += 1;
            stats.approx_tokens += (prompt.len() + reply.len()) / 4;
            if self.token_budget > 0 && stats.approx_tokens > self.token_budget {
                sink(self.note(
                    &env,
                    HopKind::Terminated,
                    format!(
                        "thread terminated: token budget {} reached (~{} tokens)",
                        self.token_budget, stats.approx_tokens
                    ),
                ));
                return stats;
            }
            match parse_routing(&reply) {
                Routing::Relay { to: next, body } => {
                    if next.eq_ignore_ascii_case(&env.to) {
                        sink(self.back(&env, HopKind::Done, body)); // self-hand-off → finish
                        return stats;
                    }
                    sink(Hop {
                        from: env.to.clone(),
                        to: next.clone(),
                        hop: env.hop,
                        kind: HopKind::Reply,
                        text: body.clone(),
                    });
                    transcript.push(format!("{} → {next}: {}", env.to, clip(&body, 400)));
                    if self.registry.get(&next).is_none() {
                        sink(self.note(&env, HopKind::Error, format!("unknown peer \"{next}\"")));
                        return stats;
                    }
                    env = env.advance(env.to.clone(), next, body);
                }
                Routing::Done(answer) => {
                    sink(self.back(&env, HopKind::Done, answer));
                    return stats;
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

/// The last few transcript entries joined — bounded context for the next agent.
fn tail(transcript: &[String]) -> String {
    const MAX: usize = 8;
    transcript[transcript.len().saturating_sub(MAX)..].join("\n")
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

#[cfg(test)]
#[path = "engine_budget_tests.rs"]
mod budget_tests;
