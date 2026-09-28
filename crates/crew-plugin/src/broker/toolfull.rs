//! A relay follow-up past the model's context, cut down and dialed once more.
//!
//! Each tool round rebuilds the follow-up from the base prompt (project card,
//! AGENTS.md, standing memory, recalled turns, the frame) and the round log,
//! and the last two rounds' results ride whole. Two or three big reads on top
//! of that reach qwen-max's 32K tokens, the provider refuses the dial
//! (`Range of input length should be [1, 30720]`), and the turn ended there
//! on an error card, everything the agent had read thrown away. Now that one
//! error cuts the prompt and dials again, once: the recalled turns leave the
//! base, the log keeps its last round whole and every older call as its
//! `CALLED` line (`Exchanges::tighten`), and the pane says so in one line.
//! A second refusal ends the turn with the error, as before.
use std::sync::Arc;

use super::adapter::{Adapter, HopStream, Usage};
use super::hop::{note, Hop, HopKind};
use super::{Broker, Envelope};
use crew_hive::tools::exchanges::{Exchanges, CONTEXT_FULL};

/// What one follow-up dial came back with, and the prompt it was sent.
pub(super) type Dialed = (String, Result<(String, Usage), String>);

/// The follow-up for a turn with `left` calls to go.
pub(super) fn follow(base: &str, log: &Exchanges, left: u32) -> String {
    format!(
        "{base}\n\nTOOL EXCHANGES THIS TURN:\n{}\n\n{}",
        log.render(),
        super::toolround::next_step(left)
    )
}

/// Cut the turn's prompt to fit: the recalled turns out of `base`, the log
/// down to its last round. `false` when neither had anything left to give,
/// and a second dial would send what was just refused.
pub(super) fn shrink(base: &mut String, log: &mut Exchanges) -> bool {
    let recalled = super::recall::cut(base);
    let cut = recalled.is_some();
    if let Some(rest) = recalled {
        *base = rest;
    }
    // Both, whatever the first gave: the log is cut for the rest of the turn.
    log.tighten() | cut
}

impl Broker {
    /// Dial `agent` with the follow-up to `base` and `log`, and when the model
    /// refuses it for its length, cut both (they stay cut for the turn), say
    /// so, and dial once more.
    ///
    /// `from` names the call the dial answers (the Dialing hop's sender), and
    /// `tick_base` the hop's running token estimate (see `run_tools`).
    #[allow(clippy::too_many_arguments)] // engine-loop plumbing, one call site
    pub(super) fn dial_fitted(
        &self,
        agent: &dyn Adapter,
        base: &mut String,
        log: &mut Exchanges,
        left: u32,
        from: &str,
        tick_base: u64,
        env: &Envelope,
        stream: &HopStream,
        sink: &mut dyn FnMut(Hop),
    ) -> Dialed {
        let sent = follow(base, log, left);
        let got = self.dial_follow(agent, &sent, from, tick_base, env, stream, sink);
        match &got {
            Err(e) if crew_hive::provider::says_context_overflow(e) && shrink(base, log) => {
                sink(note(env, HopKind::Reply, CONTEXT_FULL.into()));
                let sent = follow(base, log, left);
                let got = self.dial_follow(agent, &sent, from, tick_base, env, stream, sink);
                (sent, got)
            }
            _ => (sent, got),
        }
    }

    /// One follow-up dial: the Dialing hop that opens it, and the call, its
    /// token ticks offset by `tick_base`. Each dial restarts its own chars/4
    /// estimate at 0 and the hop's shared gate only emits on growth, so
    /// without the offset a short follow-up after a long reply never ticks.
    /// Text and reasoning need none: fragments are appended, not compared.
    #[allow(clippy::too_many_arguments)] // engine-loop plumbing, one call site
    fn dial_follow(
        &self,
        agent: &dyn Adapter,
        prompt: &str,
        from: &str,
        tick_base: u64,
        env: &Envelope,
        stream: &HopStream,
        sink: &mut dyn FnMut(Hop),
    ) -> Result<(String, Usage), String> {
        sink(Hop {
            from: from.to_string(),
            to: env.to.clone(),
            hop: env.hop,
            kind: HopKind::Dialing,
            text: String::new(),
            usage: Default::default(),
        });
        let ticked = HopStream {
            on_tokens: {
                let on = Arc::clone(&stream.on_tokens);
                Arc::new(move |t| on(tick_base + t))
            },
            on_text: Arc::clone(&stream.on_text),
            on_thought: Arc::clone(&stream.on_thought),
            on_tool: Arc::clone(&stream.on_tool),
        };
        agent.call_with_usage_ticked(prompt, self.timeout, &ticked)
    }
}

#[cfg(test)]
#[path = "toolfull_tests.rs"]
mod tests;
