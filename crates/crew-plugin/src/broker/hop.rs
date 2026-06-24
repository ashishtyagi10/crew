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
}

/// Approximate cost of a relay: agent calls made and ~tokens (chars / 4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunStats {
    pub exchanges: u32,
    pub approx_tokens: usize,
}
