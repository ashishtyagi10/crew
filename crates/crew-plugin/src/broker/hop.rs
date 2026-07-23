//! Broker value types shared by the relay engine: the per-hop transcript entry
//! and the per-thread cost stats. Split out of `engine` to keep it under the
//! line cap.

/// Why a hop was logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HopKind {
    /// About to call an agent (progress note shown before the call); `to` = it.
    Dialing,
    /// A normal reply (relayed onward or bounced back to the sender).
    Reply,
    /// The agent ended the thread with `DONE`.
    Done,
    /// The hop limit (or another guard) was reached; the thread was dropped.
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
    /// Real usage of the agent call that produced this hop (reply hops from
    /// API-backed agents); zeros for dialing/notes and usage-less backends.
    pub usage: super::adapter::Usage,
}

/// Cost of a relay: agent calls made, ~tokens (chars / 4), and — when the
/// backend reports usage — the real token total (0 = nothing reported).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunStats {
    pub exchanges: u32,
    pub approx_tokens: usize,
    pub real_tokens: usize,
    pub tok_in: u64,
    pub tok_out: u64,
    pub cost_microusd: u64,
}

use super::Envelope;

/// A hop produced by the agent at `env.to`, bound back to `env.from`.
pub(crate) fn back(env: &Envelope, kind: HopKind, text: String) -> Hop {
    Hop {
        from: env.to.clone(),
        to: env.from.clone(),
        hop: env.hop,
        kind,
        text,
        usage: Default::default(),
    }
}

/// A broker-originated note (loop guard / routing error) about `env`.
pub(crate) fn note(env: &Envelope, kind: HopKind, text: String) -> Hop {
    Hop {
        from: "broker".into(),
        to: env.to.clone(),
        hop: env.hop,
        kind,
        text,
        usage: Default::default(),
    }
}

/// The last few transcript entries joined — bounded context for the next agent.
pub(crate) fn transcript_tail(transcript: &[String]) -> String {
    const MAX: usize = 8;
    transcript[transcript.len().saturating_sub(MAX)..].join("\n")
}

impl Envelope {
    /// The next envelope one hop deeper, from `from` to `to` carrying `body`.
    pub(crate) fn advance(&self, from: String, to: String, body: String) -> Envelope {
        Envelope {
            from,
            to,
            thread_id: self.thread_id.clone(),
            hop: self.hop + 1,
            body,
        }
    }
}
