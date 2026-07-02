//! The broker engine: given a starting message, it calls the addressed agent,
//! logs the reply, and follows the routing decision — relay to a peer, reply
//! back to the sender, or finish — until the thread ends or the hop limit trips
//! the loop guard. Every hop is reported through a sink for observability.
use std::time::Duration;

use super::hop::{back, note, transcript_tail, Hop, HopKind, RunStats};
use super::route::{clip, frame, has_directive, repair_prompt};
use super::{parse_routing, Envelope, Registry, Routing};

/// Routes messages between agents in a [`Registry`], with a per-call timeout, a
/// maximum hop count, and an approximate token budget (0 = unlimited).
pub struct Broker {
    pub registry: Registry,
    pub max_hops: u32,
    pub timeout: Duration,
    pub token_budget: usize,
    /// Checked between hops; when it flips true (`/stop`), the thread ends
    /// with a Terminated hop instead of dialling the next agent.
    cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl Broker {
    pub fn new(registry: Registry, max_hops: u32, timeout: Duration) -> Self {
        Self {
            registry,
            max_hops,
            timeout,
            token_budget: 0,
            cancel: None,
        }
    }

    /// Cap a thread's approximate token spend (0 = unlimited).
    pub fn with_budget(mut self, tokens: usize) -> Self {
        self.token_budget = tokens;
        self
    }

    /// Attach a cooperative cancel flag (the `/stop` construct).
    pub fn with_cancel_flag(mut self, flag: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self {
        self.cancel = Some(flag);
        self
    }

    fn cancelled(&self) -> bool {
        self.cancel
            .as_ref()
            .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
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
        let mut last_body: Option<String> = None;
        let mut repaired = false; // at most one protocol-repair re-ask per thread
        let mut env = Envelope::new(from, to, thread_id, body);
        loop {
            if self.cancelled() {
                sink(note(
                    &env,
                    HopKind::Terminated,
                    "thread cancelled by /stop".into(),
                ));
                return stats;
            }
            if env.hop > self.max_hops {
                sink(note(
                    &env,
                    HopKind::Terminated,
                    format!("thread terminated: hop limit {} reached", self.max_hops),
                ));
                return stats;
            }
            let Some(agent) = self.registry.get(&env.to) else {
                sink(note(
                    &env,
                    HopKind::Error,
                    format!("unknown agent \"{}\"", env.to),
                ));
                return stats;
            };
            let peers = self.registry.roster_excluding(&env.to);
            let prompt = frame(&env, &peers, &task, &transcript_tail(&transcript));
            sink(note(&env, HopKind::Dialing, String::new()));
            let reply = match agent.call(&prompt, self.timeout) {
                Ok(r) if !r.trim().is_empty() => r,
                Ok(_) => {
                    sink(back(&env, HopKind::Error, "empty reply".into()));
                    return stats;
                }
                Err(e) => {
                    sink(back(&env, HopKind::Error, e));
                    return stats;
                }
            };
            stats.exchanges += 1;
            stats.approx_tokens += (prompt.len() + reply.len()) / 4;
            // If the agent forgot its control line and a hand-off is possible,
            // re-ask it once to add one (bounded to a single repair per thread).
            let reply = if !repaired && !peers.is_empty() && !has_directive(&reply) {
                repaired = true;
                let nudge = repair_prompt(&peers, &reply);
                match agent.call(&nudge, self.timeout) {
                    Ok(r) if !r.trim().is_empty() => {
                        stats.exchanges += 1;
                        stats.approx_tokens += (nudge.len() + r.len()) / 4;
                        r
                    }
                    _ => reply,
                }
            } else {
                reply
            };
            if self.token_budget > 0 && stats.approx_tokens > self.token_budget {
                sink(note(
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
                        sink(back(&env, HopKind::Done, body)); // self-hand-off → finish
                        return stats;
                    }
                    let trimmed = body.trim();
                    if !trimmed.is_empty() && last_body.as_deref() == Some(trimmed) {
                        let m = "thread terminated: no progress (a reply repeated verbatim)";
                        sink(note(&env, HopKind::Terminated, m.into()));
                        return stats;
                    }
                    last_body = Some(trimmed.to_string());
                    sink(Hop {
                        from: env.to.clone(),
                        to: next.clone(),
                        hop: env.hop,
                        kind: HopKind::Reply,
                        text: body.clone(),
                    });
                    transcript.push(format!("{} → {next}: {}", env.to, clip(&body, 400)));
                    if self.registry.get(&next).is_none() {
                        sink(note(
                            &env,
                            HopKind::Error,
                            format!("unknown peer \"{next}\""),
                        ));
                        return stats;
                    }
                    env = env.advance(env.to.clone(), next, body);
                }
                Routing::Done(answer) => {
                    sink(back(&env, HopKind::Done, answer));
                    return stats;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "engine_budget_tests.rs"]
mod budget_tests;
