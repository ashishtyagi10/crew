//! One hop's calls to its agent when the harness does not run the turn: the
//! dial itself, the one re-ask for a control line it forgot, and the hop that
//! ends the thread.
use std::sync::Arc;

use super::Broker;
use crate::broker::adapter::{Adapter, HopStream, Usage};
use crate::broker::hop::{back, Hop, HopKind, RunStats};
use crate::broker::hoptool::hop_tooler;
use crate::broker::route::repair_prompt;
use crate::broker::tick::{hop_texter, hop_thinker, hop_ticker};
use crate::broker::Envelope;
use crate::PluginEvent;

/// The live signals of a hop to `to`, each its own rate-limited emitter on
/// `tick_emit`, built fresh per call so successive dials pace independently.
pub(super) fn stream(tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>, to: &str) -> HopStream {
    HopStream {
        on_tokens: hop_ticker(tick_emit.clone(), to.to_string()),
        on_text: hop_texter(tick_emit.clone(), to.to_string()),
        on_thought: hop_thinker(tick_emit.clone(), to.to_string()),
        on_tool: hop_tooler(tick_emit.clone(), to.to_string()),
    }
}

impl Broker {
    /// Dial `agent` with `prompt`, a hop with no tools to call: the reply and
    /// its usage, or `None` when the dial failed, on an error hop.
    pub(super) fn dial(
        &self,
        agent: &dyn Adapter,
        prompt: &str,
        stats: &mut RunStats,
        env: &Envelope,
        stream: &HopStream,
        sink: &mut dyn FnMut(Hop),
    ) -> Option<(String, Usage)> {
        match agent.call_with_usage_ticked(prompt, self.timeout, stream) {
            Ok((reply, usage)) if !reply.trim().is_empty() => {
                stats.add((prompt.len() + reply.len()) / 4, &usage);
                Some((reply, usage))
            }
            Ok(_) => {
                sink(back(env, HopKind::Error, "empty reply".into()));
                None
            }
            Err(e) => {
                sink(back(env, HopKind::Error, e));
                None
            }
        }
    }

    /// The hop that ends the thread with `answer`: the one place a relay
    /// turn's final answer is made, so the checklist's line lands under every
    /// answer however the thread ended (`toolround::finished`).
    pub(super) fn done(&self, env: &Envelope, answer: String, usage: Usage) -> Hop {
        let answer = crate::broker::toolround::finished(self.tools.as_deref(), answer);
        Hop {
            usage,
            ..back(env, HopKind::Done, answer)
        }
    }

    /// `reply` with the control line it forgot: `agent` asked once to add
    /// one, on `stream`, and its answer kept when it gave one.
    pub(super) fn repair(
        &self,
        agent: &dyn Adapter,
        peers: &[String],
        reply: String,
        usage: &mut Usage,
        stats: &mut RunStats,
        stream: &HopStream,
    ) -> String {
        let nudge = repair_prompt(peers, &reply);
        match agent.call_with_usage_ticked(&nudge, self.timeout, stream) {
            Ok((r, u)) if !r.trim().is_empty() => {
                stats.add((nudge.len() + r.len()) / 4, &u);
                usage.latest(u); // the repair call's context is the latest
                r
            }
            _ => reply,
        }
    }
}
