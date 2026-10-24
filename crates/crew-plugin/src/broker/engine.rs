//! The broker engine: given a starting message, it calls the addressed agent,
//! logs the reply, and follows the routing decision — relay to a peer, reply
//! back to the sender, or finish — until the thread ends or the hop limit trips
//! the loop guard. Every hop is reported through a sink for observability.
use std::sync::Arc;

use super::compact::{is_dup, keep_in_transcript, Compactor};
use super::hop::{note, Hop, HopKind, RunStats};
use super::route::{clip, frame, has_directive, intro_of};
use super::{parse_routing, Envelope, Routing};
use crate::PluginEvent;

#[path = "enginebuild.rs"]
mod build;
#[path = "enginedial.rs"]
mod dial;

pub use build::Broker;

impl Broker {
    /// Drive a relay from `from` to `to`; hops stream through `sink`. Every
    /// agent hop also gets its own rate-limited `StatsTick` emitter, built
    /// fresh per call so successive hops (and the retry below) pace
    /// independently — see [`super::tick::hop_ticker`].
    pub fn run(
        &self,
        from: &str,
        to: &str,
        body: &str,
        thread_id: &str,
        tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
        sink: &mut dyn FnMut(Hop),
    ) -> RunStats {
        let tools = super::toolcall::hint(body, self.tools.as_deref());
        let mut transcript: Vec<String> = Vec::new();
        let mut compact = Compactor::new(self.summarizer.clone());
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
            let Some(shared) = self.registry.shared(&env.to) else {
                sink(note(
                    &env,
                    HopKind::Error,
                    format!("unknown agent \"{}\"", env.to),
                ));
                return stats;
            };
            let agent = shared.as_ref();
            let peers = self.registry.roster_excluding(&env.to);
            let tail = compact.tail(&mut transcript);
            let prompt = frame(&env, intro_of(agent), &peers, body, &tools, &tail);
            // The dial names its real sender (`user`, or the relaying peer) so
            // the host's activity row can show who the agent is working for.
            sink(Hop {
                from: env.from.clone(),
                to: env.to.clone(),
                hop: env.hop,
                kind: HopKind::Dialing,
                text: String::new(),
                usage: Default::default(),
            });
            let stream = dial::stream(tick_emit, &env.to);
            // With tools on, the agent's whole turn runs on the harness, the
            // swarm's loop (`hopturn`); with none, it is one dial.
            let dialed = match self.tools.is_some() {
                true => {
                    let bare = frame(&env, intro_of(agent), &peers, body, "", &tail);
                    self.harnessed(shared, &env, body, bare, prompt, &mut stats, &stream, sink)
                }
                false => self.dial(agent, &prompt, &mut stats, &env, &stream, sink),
            };
            let Some((reply, mut usage)) = dialed else {
                return stats;
            };
            // If the agent forgot its control line and a hand-off is possible,
            // re-ask it once to add one (bounded to a single repair per thread)
            // — but only mid-chain. An agent answering the USER with no
            // directive has answered: routing already reads bare prose as
            // done, and the re-ask cost a second full call, typing the same
            // answer out twice, to add one word nobody reads. Nor after a
            // stop: a reply the stop cut short has no directive to add.
            let answering_user = env.from == "user";
            let open = !answering_user && !peers.is_empty() && !self.cancelled();
            let reply = if !repaired && open && !has_directive(&reply) {
                repaired = true;
                let stream = dial::stream(tick_emit, &env.to);
                self.repair(agent, &peers, reply, &mut usage, &mut stats, &stream)
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
                    // A hand-off to nobody on the roster — an agent alone on a
                    // routed reply, or a name remembered from another turn — is
                    // the answer, not an error card.
                    if next.eq_ignore_ascii_case(&env.to) || self.registry.get(&next).is_none() {
                        sink(self.done(&env, body, usage)); // self-hand-off → finish
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
                        usage,
                    });
                    if keep_in_transcript(&body) {
                        let entry = format!("{} → {next}: {}", env.to, clip(&body, 400));
                        if !is_dup(&transcript, &entry) {
                            transcript.push(entry);
                        }
                    }
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
                    sink(self.done(&env, answer, usage));
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

#[cfg(test)]
#[path = "toolreach_tests.rs"]
mod toolreach_tests;
